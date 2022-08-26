use async_trait::async_trait;
use crd::{Backend, RemoteValue, SecretData};

use k8s_openapi::ByteString;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct PlainText {
    data: Vec<SecretData>,
}

#[async_trait]
impl RemoteValue for PlainText {
    fn from_backend(backend: &Backend) -> PlainText {
        PlainText {
            data: backend.data.clone(),
        }
    }

    async fn get_value(&self) -> BTreeMap<String, ByteString> {
        let mut secrets = BTreeMap::new();

        for secret_data in self.data.iter() {
            if secret_data.secret_field_name.is_some() {
                let key = secret_data.secret_field_name.clone().unwrap();

                secrets.insert(
                    key,
                    ByteString(secret_data.remote_value.clone().as_bytes().to_vec()),
                );
            }
        }

        secrets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[tokio::test]
    async fn test_plain_text() {
        let backend_str = r#"
        {
            "backend": "Plaintext",
            "data": [
                {
                    "remote_value": "test1",
                    "secret_field_name": "value1"
                },
                {
                    "remote_value": "test2",
                    "secret_field_name": "value2"
                }
            ]
        }"#;

        let backend: Backend = serde_json::from_str(backend_str).unwrap();

        let plaintext = PlainText::from_backend(&backend);

        let result = plaintext.get_value();

        let _value = result.await;

        assert!(true);
    }
}
