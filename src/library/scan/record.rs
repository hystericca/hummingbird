use std::{io::ErrorKind, path::Path, sync::Arc, time::SystemTime};

use async_compression::tokio::bufread::ZlibDecoder;
use async_compression::tokio::write::ZlibEncoder;
use camino::Utf8PathBuf;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};
use tracing::{error, info};

/// The version of the scanning process. If this version number is incremented, a re-scan of all
/// files will be forced (see [ScanCommand::ForceScan]).
pub const SCAN_VERSION: u16 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRecord {
    pub version: u16,
    pub records: FxHashMap<Utf8PathBuf, SystemTime>,
    pub directories: Vec<Utf8PathBuf>,
}

impl ScanRecord {
    pub fn new_current() -> Self {
        Self {
            version: SCAN_VERSION,
            records: FxHashMap::default(),
            directories: Vec::new(),
        }
    }

    pub fn is_version_mismatch(&self) -> bool {
        self.version != SCAN_VERSION
    }
}

pub async fn load_scan_record(path: &Path) -> ScanRecord {
    let mut file = match tokio::fs::File::open(path)
        .await
        .map(BufReader::new)
        .map(ZlibDecoder::new)
    {
        Ok(f) => f,
        Err(e) => {
            if e.kind() != ErrorKind::NotFound {
                error!("Could not open scan record: {:?}", e);
                error!("Scanning will be slow until the scan record is rebuilt");
            }

            return ScanRecord::new_current();
        }
    };

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await.unwrap_or_default();

    match postcard::from_bytes(&bytes) {
        Ok(scan_record) => scan_record,
        Err(e) => {
            error!("Could not read scan record: {:?}", e);
            error!("Scanning will be slow until the scan record is rebuilt");
            ScanRecord::new_current()
        }
    }
}

#[derive(Serialize)]
struct ScanRecordForWrite<'a> {
    version: u16,
    records: &'a FxHashMap<Utf8PathBuf, SystemTime>,
    directories: &'a [Utf8PathBuf],
}

pub async fn write_checkpoint(
    checkpoint: Arc<Mutex<FxHashMap<Utf8PathBuf, SystemTime>>>,
    directories: Vec<Utf8PathBuf>,
    path: &Path,
) {
    let tmp_path = path.with_extension("hsr.tmp");

    let serialized = {
        let guard = checkpoint.lock().await;
        let view = ScanRecordForWrite {
            version: SCAN_VERSION,
            records: &*guard,
            directories: &directories,
        };
        postcard::to_allocvec(&view)
    };

    let data = match serialized {
        Ok(d) => d,
        Err(e) => {
            error!("Could not serialize scan record checkpoint: {:?}", e);
            return;
        }
    };

    let mut file = match tokio::fs::File::create(&tmp_path)
        .await
        .map(ZlibEncoder::new)
    {
        Ok(f) => f,
        Err(e) => {
            error!("Could not create scan record checkpoint file: {:?}", e);
            return;
        }
    };

    if let Err(e) = file.write_all(&data).await {
        error!("Could not write scan record checkpoint: {:?}", e);
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return;
    }
    if let Err(e) = file.shutdown().await {
        error!("Could not close scan record checkpoint: {:?}", e);
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return;
    }
    if let Err(e) = tokio::fs::rename(&tmp_path, path).await {
        error!(
            "Could not rename scan record checkpoint into place: {:?}",
            e
        );
        let _ = tokio::fs::remove_file(&tmp_path).await;
    }
}

pub async fn write_scan_record(scan_record: &ScanRecord, path: &Path) {
    let tmp_path = path.with_extension("hsr.tmp");

    let mut file = match tokio::fs::File::create(&tmp_path)
        .await
        .map(ZlibEncoder::new)
    {
        Ok(file) => file,
        Err(e) => {
            error!("Could not create temporary scan record file: {:?}", e);
            error!("Scan record will not be saved, this may cause rescans on restart");
            return;
        }
    };

    match postcard::to_allocvec(&scan_record) {
        Ok(data) => {
            if let Err(e) = file.write_all(&data).await {
                error!("Could not write scan record: {:?}", e);
                error!("Scan record will not be saved, this may cause rescans on restart");
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return;
            }

            if let Err(e) = file.shutdown().await {
                error!("Could not close scan record: {:?}", e);
                error!("Scan record will not be saved, this may cause rescans on restart");
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return;
            }

            if let Err(e) = tokio::fs::rename(&tmp_path, path).await {
                error!("Could not rename scan record into place: {:?}", e);
                error!("Scan record will not be saved, this may cause rescans on restart");
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return;
            }

            info!("Scan record saved successfully");
        }
        Err(e) => {
            error!("Could not serialize scan record: {:?}", e);
            error!("Scan record will not be saved, this may cause rescans on restart");
            let _ = tokio::fs::remove_file(&tmp_path).await;
        }
    }
}
