//! Since none of the VDF implementations I tried on crates.io worked for my intended purposes, I wrote my own.
//!
//! Considering that this is a very badly documented data format, some data types (such as booleans) were implemented
//! in a way that looks compatible with the format (much like an extension, in case it was not intended).
//!
//! # Usage
//!
//! ```rust
//!
//! ```

mod deserializer;
mod error;
mod preprocessor;
mod serializer;

pub use deserializer::{from_str, Deserializer};
pub use error::{Error, Result};
pub use serializer::{to_string, Serializer};

pub(crate) use preprocessor::{peek_expect_char, preprocess};

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn it_works() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Data {
            name: String,
            list_str: Vec<String>,
            map: std::collections::HashMap<String, i64>,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Test {
            int: u32,
            seq: Vec<Data>,
        }
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestContainer {
            test: Test,
        }

        let test = Test {
            int: 1,
            seq: vec![
                Data {
                    name: "Better VDF".to_string(),
                    list_str: vec![
                        "value1".to_string(),
                        "value2".to_string(),
                        "value3".to_string(),
                    ],
                    map: [
                        ("zbx".to_string(), 12318293),
                        ("thc".to_string(), -12393180),
                    ]
                    .iter()
                    .cloned()
                    .collect(),
                },
                Data {
                    name: "rrrrr".to_string(),
                    list_str: vec![
                        "1243".to_string(),
                        "sadferw".to_string(),
                        "batebt".to_string(),
                    ],
                    map: vec![("abc".to_string(), 444444), ("key".to_string(), -555555)]
                        .iter()
                        .cloned()
                        .collect(),
                },
            ],
        };
        let result = TestContainer { test };

        let result_str = to_string(&result).unwrap();
        println!("Result:\n{}", result_str);

        // let deserialized: TestContainer =
        //     from_str(format!("{{\n{}\n}}", &result_str).as_str()).unwrap();
        let deserialized: TestContainer = from_str(&result_str).unwrap();
        println!("{:#?}", deserialized);
        assert_eq!(result, deserialized);

        // let expected_str = to_string(&deserialized).unwrap();

        // println!("Expected:\n{}", expected_str);

        // // Remove all whitespace for assertion test.
        // assert_eq!(
        //     result_str.replace(|c: char| c.is_whitespace(), ""),
        //     expected_str.replace(|c: char| c.is_whitespace(), "")
        // );
    }
}
