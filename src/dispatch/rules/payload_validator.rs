//! Payload map validators for `PayloadMapRule`

use std::sync::Arc;

use serde_json::Value;

/// Validator for a single payload field
pub enum PayloadValidator {
    Equals(Value),
    Nested(Vec<(String, PayloadValidator)>),
    Func(Arc<dyn Fn(&Value) -> bool + Send + Sync>),
}

impl PayloadValidator {
    pub fn equals(value: Value) -> Self {
        Self::Equals(value)
    }

    pub fn nested(fields: Vec<(String, PayloadValidator)>) -> Self {
        Self::Nested(fields)
    }

    pub fn func<F>(f: F) -> Self
    where
        F: Fn(&Value) -> bool + Send + Sync + 'static,
    {
        Self::Func(Arc::new(f))
    }

    pub fn check(&self, value: &Value) -> bool {
        match self {
            Self::Equals(expected) => value == expected,
            Self::Nested(fields) => {
                let Some(obj) = value.as_object() else {
                    return false;
                };
                for (key, validator) in fields {
                    match obj.get(key) {
                        Some(v) if validator.check(v) => {}
                        _ => return false,
                    }
                }
                true
            }
            Self::Func(f) => f(value),
        }
    }
}

/// Build validators from a JSON map (shortcut for labeler)
pub fn validators_from_json(map: &serde_json::Map<String, Value>) -> Vec<(String, PayloadValidator)> {
    map.iter()
        .map(|(k, v)| {
            let validator = if let Some(nested) = v.as_object() {
                let inner = validators_from_json(nested);
                PayloadValidator::nested(inner)
            } else {
                PayloadValidator::equals(v.clone())
            };
            (k.clone(), validator)
        })
        .collect()
}

/// Match full payload object against validator list
pub fn match_payload_map(payload: &Value, validators: &[(String, PayloadValidator)]) -> bool {
    let Some(obj) = payload.as_object() else {
        return false;
    };
    for (key, validator) in validators {
        match obj.get(key) {
            Some(v) if validator.check(v) => {}
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nested_payload_match() {
        let validators = vec![
            (
                "cmd".to_string(),
                PayloadValidator::equals(json!("start")),
            ),
            (
                "meta".to_string(),
                PayloadValidator::nested(vec![(
                    "id".to_string(),
                    PayloadValidator::equals(json!(1)),
                )]),
            ),
        ];
        let payload = json!({"cmd": "start", "meta": {"id": 1}});
        assert!(match_payload_map(&payload, &validators));
    }
}
