/// Helpers for unwrapping CozoDB DataValue JSON serialization.
///
/// CozoDB serializes values as tagged enums:
/// - `{"Str": "value"}` → string
/// - `{"Bool": true}` → bool
/// - `{"Num": {"Int": N}}` → i64
/// - `{"Num": {"Float": F}}` → f64
/// - `"Null"` → None
///
/// These helpers extract the inner value from the tagged wrapper,
/// also accepting plain JSON values for forward compatibility.

/// Extract a string from a CozoDB DataValue JSON.
pub fn as_str(v: &serde_json::Value) -> Option<&str> {
    // Tagged: {"Str": "..."}
    if let Some(s) = v.get("Str").and_then(|s| s.as_str()) {
        return Some(s);
    }
    // Plain string
    v.as_str()
}

/// Extract a bool from a CozoDB DataValue JSON.
pub fn as_bool(v: &serde_json::Value) -> Option<bool> {
    // Tagged: {"Bool": true/false}
    if let Some(b) = v.get("Bool").and_then(|b| b.as_bool()) {
        return Some(b);
    }
    // Plain bool
    v.as_bool()
}

/// Extract an i64 from a CozoDB DataValue JSON.
pub fn as_i64(v: &serde_json::Value) -> Option<i64> {
    // Tagged: {"Num": {"Int": N}}
    if let Some(num) = v.get("Num") {
        if let Some(i) = num.get("Int").and_then(|i| i.as_i64()) {
            return Some(i);
        }
        // Could also be {"Num": {"Float": F}}
        if let Some(f) = num.get("Float").and_then(|f| f.as_f64()) {
            return Some(f as i64);
        }
    }
    // Plain number
    v.as_i64()
}
