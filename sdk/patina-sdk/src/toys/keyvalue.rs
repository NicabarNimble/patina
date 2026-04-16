pub struct Bucket {
    inner: crate::types::wasi::keyvalue::store::Bucket,
}

pub fn open(identifier: &str) -> Result<Bucket, String> {
    let bucket = crate::types::wasi::keyvalue::store::open(identifier).map_err(error_to_string)?;
    Ok(Bucket { inner: bucket })
}

impl Bucket {
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        self.inner.get(key).map_err(error_to_string)
    }

    pub fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
        self.inner.set(key, value).map_err(error_to_string)
    }

    pub fn exists(&self, key: &str) -> Result<bool, String> {
        self.inner.exists(key).map_err(error_to_string)
    }

    pub fn drop(self) {}
}

fn error_to_string(error: crate::types::wasi::keyvalue::store::Error) -> String {
    match error {
        crate::types::wasi::keyvalue::store::Error::NoSuchStore => "no-such-store".to_string(),
        crate::types::wasi::keyvalue::store::Error::AccessDenied => "access-denied".to_string(),
        crate::types::wasi::keyvalue::store::Error::Other(message) => format!("other({message})"),
    }
}
