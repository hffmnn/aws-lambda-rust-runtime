use std::{borrow::Cow, collections::HashMap, fmt};

use serde::{
    de::{Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

/// `IamPolicyDocument` represents an IAM policy document.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct IamPolicyDocument {
    #[serde(default)]
    pub version: Option<String>,
    pub statement: Vec<IamPolicyStatement>,
}

/// `IamPolicyStatement` represents one statement from IAM policy with action, effect and resource
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct IamPolicyStatement {
    #[serde(deserialize_with = "deserialize_string_or_slice")]
    pub action: Vec<String>,
    #[serde(default = "default_statement_effect")]
    pub effect: IamPolicyEffect,
    #[serde(deserialize_with = "deserialize_string_or_slice")]
    pub resource: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_policy_condition")]
    pub condition: Option<IamPolicyCondition>,
}

pub type IamPolicyCondition = HashMap<String, HashMap<String, Vec<String>>>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum IamPolicyEffect {
    #[default]
    Allow,
    Deny,
}

fn default_statement_effect() -> IamPolicyEffect {
    IamPolicyEffect::Allow
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum StringOrSlice {
    String(String),
    Slice(Vec<String>),
}

/// Deserializes `Vec<String>`, from a JSON `string` or `[string]`.
fn deserialize_string_or_slice<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let string_or_slice = StringOrSlice::deserialize(deserializer)?;

    match string_or_slice {
        StringOrSlice::Slice(slice) => Ok(slice),
        StringOrSlice::String(s) => Ok(vec![s]),
    }
}

fn deserialize_policy_condition<'de, D>(de: D) -> Result<Option<IamPolicyCondition>, D::Error>
where
    D: Deserializer<'de>,
{
    de.deserialize_option(IamPolicyConditionVisitor)
}

struct IamPolicyConditionVisitor;

impl<'de> Visitor<'de> for IamPolicyConditionVisitor {
    type Value = Option<IamPolicyCondition>;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("lots of things can go wrong with a IAM Policy Condition")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(None)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((key, val)) = access.next_entry::<Cow<'_, str>, HashMap<Cow<'_, str>, StringOrSlice>>()? {
            let mut value = HashMap::with_capacity(val.len());
            for (val_key, string_or_slice) in val {
                let val = match string_or_slice {
                    StringOrSlice::Slice(slice) => slice,
                    StringOrSlice::String(s) => vec![s],
                };
                value.insert(val_key.into_owned(), val);
            }

            map.insert(key.into_owned(), value);
        }

        Ok(Some(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_string_condition() {
        let data = serde_json::json!({
            "condition": {
                "StringEquals": {
                    "iam:RegisterSecurityKey": "Activate",
                    "iam:FIDO-certification": "L1plus"
                }
            }
        });

        #[derive(Deserialize)]
        struct Test {
            #[serde(deserialize_with = "deserialize_policy_condition")]
            condition: Option<IamPolicyCondition>,
        }

        let test: Test = serde_json::from_value(data).unwrap();
        let condition = test.condition.unwrap();
        assert_eq!(1, condition.len());

        assert_eq!(vec!["Activate"], condition["StringEquals"]["iam:RegisterSecurityKey"]);
        assert_eq!(vec!["L1plus"], condition["StringEquals"]["iam:FIDO-certification"]);
    }

    #[test]
    fn test_deserialize_slide_condition() {
        let data = serde_json::json!({
            "condition": {"StringLike": {"s3:prefix": ["janedoe/*"]}}
        });

        #[derive(Deserialize)]
        struct Test {
            #[serde(deserialize_with = "deserialize_policy_condition")]
            condition: Option<IamPolicyCondition>,
        }

        let test: Test = serde_json::from_value(data).unwrap();
        let condition = test.condition.unwrap();
        assert_eq!(1, condition.len());

        assert_eq!(vec!["janedoe/*"], condition["StringLike"]["s3:prefix"]);
    }
}
