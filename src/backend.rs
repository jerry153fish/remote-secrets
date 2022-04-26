use crate::{finalizer, BackendType, Metrics, RSecret};
use k8s_openapi::ByteString;
use std::collections::BTreeMap;

pub async fn get_secret_data(
    rsecret: &RSecret,
    data: BTreeMap<String, ByteString>,
) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();
    rsecret
        .spec
        .resources
        .iter()
        .for_each(|backend| match backend.backend {
            BackendType::Plaintext => {
                backend.data.iter().for_each(|secret_data| {
                    if secret_data.secret_field_name.is_some() {
                        secrets.insert(
                            secret_data.secret_field_name.clone().unwrap(),
                            ByteString(secret_data.name_or_value.clone().as_bytes().to_vec()),
                        );
                    }
                });
            }
            _ => {}
        });

    return secrets.into_iter().chain(data.into_iter()).collect();
}
