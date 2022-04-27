use crate::{aws, finalizer, Backend, BackendType, Metrics, RSecret};
use k8s_openapi::ByteString;
use std::collections::BTreeMap;

pub async fn get_secret_data(
    rsecret: &RSecret,
    data: BTreeMap<String, ByteString>,
) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();

    for backend in rsecret.spec.resources.iter() {
        match backend.backend {
            BackendType::Plaintext => {
                let plain_text_secret_data = get_plain_text_secret_data(backend);
                secrets = plain_text_secret_data
                    .into_iter()
                    .chain(secrets.clone().into_iter())
                    .collect();
            }
            BackendType::SecretManager => {
                let secret_manager_secret_data =
                    get_secret_manager_secret_data(backend).await.unwrap();
                secrets = secret_manager_secret_data
                    .into_iter()
                    .chain(secrets.clone().into_iter())
                    .collect();
            }
            BackendType::SSM => {
                let aws_ssm_data = get_ssm_secret_data(backend).await.unwrap();
                secrets = aws_ssm_data
                    .into_iter()
                    .chain(secrets.clone().into_iter())
                    .collect();
            }
            _ => {}
        };
    }

    return secrets.into_iter().chain(data.into_iter()).collect();
}

pub fn get_plain_text_secret_data(backend: &Backend) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();
    backend.data.iter().for_each(|secret_data| {
        if secret_data.secret_field_name.is_some() {
            secrets.insert(
                secret_data.secret_field_name.clone().unwrap(),
                ByteString(secret_data.name_or_value.clone().as_bytes().to_vec()),
            );
        }
    });

    secrets
}

pub async fn get_secret_manager_secret_data(
    backend: &Backend,
) -> Result<BTreeMap<String, ByteString>, aws_sdk_secretsmanager::Error> {
    let mut secrets = BTreeMap::new();

    for secret_data in backend.data.iter() {
        if secret_data.secret_field_name.is_some() {
            let ssm_secret_data =
                aws::get_secretsmanager_parameter(secret_data.name_or_value.clone()).await;

            let key = secret_data.secret_field_name.clone().unwrap();

            match ssm_secret_data {
                Ok(ssm_secret_data) => {
                    secrets.insert(key, ByteString(ssm_secret_data.clone().into_bytes()));
                }
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }
    }

    Ok(secrets)
}

pub async fn get_ssm_secret_data(
    backend: &Backend,
) -> Result<BTreeMap<String, ByteString>, aws_sdk_secretsmanager::Error> {
    let mut secrets = BTreeMap::new();

    for secret_data in backend.data.iter() {
        if secret_data.secret_field_name.is_some() {
            let ssm_secret_data = aws::get_ssm_parameter(secret_data.name_or_value.clone()).await;

            let key = secret_data.secret_field_name.clone().unwrap();

            match ssm_secret_data {
                Ok(ssm_secret_data) => {
                    secrets.insert(key, ByteString(ssm_secret_data.clone().into_bytes()));
                }
                Err(err) => {
                    log::error!("{}", err);
                }
            }
        }
    }

    Ok(secrets)
}
