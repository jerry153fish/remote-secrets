use anyhow::Result;
use crd::SecretData;
use json_dotpath::DotPaths;
use k8s_openapi::ByteString;
use serde_json::Value;
use std::collections::BTreeMap;

pub fn get_json_string_nested_value(json_string: &str, path: &str) -> Result<String> {
    let json: Value = serde_json::from_str(json_string)?;

    let result = json
        .dot_get::<String>(path)
        .unwrap_or_default()
        .unwrap_or_default();

    Ok(result)
}

pub fn get_json_string_as_secret_data(json_string: &str) -> Result<BTreeMap<String, ByteString>> {
    let json: Value = serde_json::from_str(json_string)?;
    let mut secrets = BTreeMap::new();
    let json_as_hashmap = json.as_object().unwrap();
    for (key, value) in json_as_hashmap {
        secrets.insert(
            key.to_owned(),
            ByteString(value.as_str().unwrap().to_owned().as_bytes().to_vec()),
        );
    }

    Ok(secrets)
}

// TODO: a better way error handling
pub fn get_secret_data(
    rsecret_data: &SecretData,
    value_string: &str,
) -> BTreeMap<String, ByteString> {
    let mut secrets = BTreeMap::new();
    if rsecret_data.key.is_some() {
        let key = rsecret_data.key.clone().unwrap();

        if rsecret_data.is_json_string.unwrap_or_default() {
            if rsecret_data.remote_path.is_some() {
                let value = get_json_string_nested_value(
                    value_string,
                    &rsecret_data.remote_path.clone().unwrap(),
                );

                match value {
                    Ok(value) => {
                        if !value.is_empty() {
                            secrets.insert(key, ByteString(value.as_bytes().to_vec()));
                        }
                    }
                    Err(e) => {
                        log::error!("{}", e);
                    }
                }
            }
        } else {
            secrets.insert(key, ByteString(value_string.as_bytes().to_vec()));
        }
    } else {
        let data = get_json_string_as_secret_data(value_string);
        match data {
            Ok(data) => {
                secrets = data;
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }
    }

    secrets
}

pub fn merge_secret_data(
    to_merge: BTreeMap<String, ByteString>,
    merged: BTreeMap<String, ByteString>,
) -> BTreeMap<String, ByteString> {
    to_merge.into_iter().chain(merged.into_iter()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_json_string_nested_value() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ],
            "address": {
                "street": "Downing Street 10"
            }
        }"#;

        let name: String = get_json_string_nested_value(data, "name").unwrap();
        let street: String = get_json_string_nested_value(data, "address.street").unwrap();
        let phone1: String = get_json_string_nested_value(data, "phones.0").unwrap();
        let not_existed: String = get_json_string_nested_value(data, "notExisted").unwrap();

        assert_eq!(name, "John Doe");
        assert_eq!(street, "Downing Street 10");
        assert_eq!(phone1, "+44 1234567");
        assert_eq!(not_existed, "");
    }

    #[test]
    fn test_get_secret_data() {
        let r_data_raw = r#"
        {
            "value": "test"
        }"#;

        let value_string = r#"
        {
            "key": "test"
        }"#;

        let r_data: SecretData = serde_json::from_str(r_data_raw).unwrap();

        let result = get_secret_data(&r_data, &value_string);

        println!("{:?}", result);
    }
}
