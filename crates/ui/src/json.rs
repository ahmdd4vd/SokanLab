// SokanLab ui — parser JSON minimal (tanpa serde) untuk layout persist.
// Nilai didukung: null/bool/number/string/array/object. Angka disimpan f64;
// timestamp millis (< 2^53) dan tick harga aman eksak.
// Ini bukan parser JSON lengkap-per-spek untuk dipakai umum, cukup untuk berkas
// layout milik SokanLab sendiri (dan tetap menolak input rusak, bukan panic).

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// Konversi integer; menolak NaN/inf/tak-eksak (mis. 1.5 → tetap 1, seperti Rust `as`).
    pub fn as_i64(&self) -> Option<i64> {
        let n = self.as_f64()?;
        if n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            Some(n as i64)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(a) => Some(a),
            _ => None,
        }
    }
}

/// Escape string agar aman dimasukkan ke JSON.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Parse satu nilai JSON; menolak karakter sisa (strict).
pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = P {
        c: text.chars().collect(),
        i: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.c.len() {
        return Err(format!("ada karakter sisa di posisi {}", p.i));
    }
    Ok(v)
}

struct P {
    c: Vec<char>,
    i: usize,
}

impl P {
    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn expect(&mut self, want: char) -> Result<(), String> {
        if self.peek() == Some(want) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!(
                "harap '{}' di posisi {}, dapat {:?}",
                want,
                self.i,
                self.peek()
            ))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.ws();
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => self.string().map(Value::Str),
            Some('t') => self.literal("true", Value::Bool(true)),
            Some('f') => self.literal("false", Value::Bool(false)),
            Some('n') => self.literal("null", Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            other => Err(format!(
                "nilai tidak dikenal di posisi {}: {other:?}",
                self.i
            )),
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, String> {
        for want in word.chars() {
            if self.peek() != Some(want) {
                return Err(format!("literal '{word}' tidak cocok di posisi {}", self.i));
            }
            self.i += 1;
        }
        Ok(v)
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.i;
        if self.peek() == Some('-') {
            self.i += 1;
        }
        let mut saw_digit = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                saw_digit = true;
                self.i += 1;
            } else if c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                self.i += 1;
            } else {
                break;
            }
        }
        let s: String = self.c[start..self.i].iter().collect();
        if !saw_digit {
            return Err(format!("angka tidak valid di posisi {start}: {s:?}"));
        }
        s.parse::<f64>()
            .map(Value::Num)
            .map_err(|_| format!("angka tidak valid: {s}"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or("string tidak ditutup")?;
            self.i += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = self.peek().ok_or("escape tidak lengkap")?;
                    self.i += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'u' => {
                            if self.i + 4 > self.c.len() {
                                return Err("\\u butuh 4 digit hex".into());
                            }
                            let hex: String = self.c[self.i..self.i + 4].iter().collect();
                            self.i += 4;
                            let n = u32::from_str_radix(&hex, 16)
                                .map_err(|_| format!("hex \\u tidak valid: {hex}"))?;
                            let ch = char::from_u32(n)
                                .ok_or_else(|| format!("codepoint \\u{hex} tidak valid"))?;
                            out.push(ch);
                        }
                        other => return Err(format!("escape tidak dikenal: \\{other}")),
                    }
                }
                c => out.push(c),
            }
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        let mut out = Vec::new();
        self.ws();
        if self.peek() == Some(']') {
            self.i += 1;
            return Ok(Value::Arr(out));
        }
        loop {
            out.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some(']') => {
                    self.i += 1;
                    return Ok(Value::Arr(out));
                }
                other => {
                    return Err(format!(
                        "array: harap ',' atau ']' di posisi {}, dapat {other:?}",
                        self.i
                    ))
                }
            }
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        let mut out: Vec<(String, Value)> = Vec::new();
        self.ws();
        if self.peek() == Some('}') {
            self.i += 1;
            return Ok(Value::Obj(out));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            self.expect(':')?;
            let val = self.value()?;
            out.push((key, val));
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some('}') => {
                    self.i += 1;
                    return Ok(Value::Obj(out));
                }
                other => {
                    return Err(format!(
                        "object: harap ',' atau '}}' di posisi {}, dapat {other:?}",
                        self.i
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_object() {
        let v = parse(r#"{"a":1,"b":[true,null,"x"],"c":{"d":-2.5}}"#).unwrap();
        assert_eq!(v.get("a").unwrap().as_i64(), Some(1));
        let b = v.get("b").unwrap().as_array().unwrap();
        assert_eq!(b.len(), 3);
        assert_eq!(b[2].as_str(), Some("x"));
        assert_eq!(v.get("c").unwrap().get("d").unwrap().as_f64(), Some(-2.5));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse("{} x").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn rejects_broken_strings() {
        assert!(parse(r#""abc"#).is_err());
        assert!(parse(r#""a\q""#).is_err());
        assert!(parse(r#""a\u12""#).is_err());
    }

    #[test]
    fn string_escapes_roundtrip() {
        let s = "a\"b\\c\nd";
        let v = parse(&format!("\"{}\"", escape(s))).unwrap();
        assert_eq!(v.as_str(), Some(s));
    }

    #[test]
    fn numbers_guard_nan_and_range() {
        assert_eq!(Value::Num(f64::NAN).as_i64(), None);
        assert_eq!(Value::Num(1e30).as_i64(), None);
        assert_eq!(Value::Num(-3.9).as_i64(), Some(-3));
    }

    #[test]
    fn object_get_is_first_match() {
        let v = parse(r#"{"k":1,"k":2}"#).unwrap();
        assert_eq!(v.get("k").unwrap().as_i64(), Some(1));
        assert!(v.get("nope").is_none());
    }
}
