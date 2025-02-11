use std::ops::{AddAssign, MulAssign, Neg};
use std::str::FromStr;

use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::preprocessor::{parse_string, peek_real_char};
use crate::{peek_expect_char, preprocess};

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
#[allow(dead_code)]
enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

#[cfg(debug_assertions)]
static mut INDENTATION: usize = 0;

#[cfg(debug_assertions)]
static mut CURRENT_LEVEL: LogLevel = LogLevel::Debug;

macro_rules! log {
    ($level:expr, $($arg:tt)*) => ({
        #[cfg(debug_assertions)]
        {
            let current_level = unsafe { CURRENT_LEVEL };
            if $level <= current_level {
                let indentation = unsafe { "\t".repeat(INDENTATION) };
                println!("{}{}", indentation, format!($($arg)*));
            }
        }
    });
}

macro_rules! adjust_indentation {
    ($delta:expr) => {
        #[cfg(debug_assertions)]
        {
            unsafe {
                let delta: i32 = $delta;
                if delta < 0 {
                    INDENTATION -= delta.abs() as usize;
                } else {
                    INDENTATION += delta as usize;
                }
            }
        }
    };
}

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de str,
    array_key: Option<String>,
    beginning: bool,
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    pub fn from_str(input: &'de str) -> Self {
        let is_properly_blocked = peek_expect_char(input, 0, '{').unwrap_or(false);
        let starts_with_header = peek_expect_char(input, 0, '"').unwrap_or(false);
        let mut parsed_input: String = input.to_string();
        if is_properly_blocked || starts_with_header {
            let mut block_preprocessor = false;
            if starts_with_header {
                let temp = parse_string(input).unwrap();
                if !peek_expect_char(temp.1, 0, '{').unwrap_or(false) {
                    block_preprocessor = true;
                }
            }
            if !block_preprocessor {
                parsed_input = preprocess(input, starts_with_header, true).unwrap();
                if starts_with_header {
                    let lines = parsed_input.lines().collect::<Vec<&str>>();
                    let mut temp_parsed_input = String::new();
                    for line in lines {
                        temp_parsed_input += "\t";
                        temp_parsed_input += line;
                        temp_parsed_input += "\n";
                    }
                    parsed_input = format!("{{\n{}\n}}", temp_parsed_input);
                }
                log!(
                    LogLevel::Debug,
                    "Preprocessed input: {:?}\n{}",
                    parsed_input,
                    parsed_input
                );
                log!(LogLevel::Debug, "-------------------");
            }
        }
        Deserializer {
            input: Box::leak(parsed_input.into_boxed_str()),
            array_key: None,
            beginning: true,
        }
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_str<'a, T>(input: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(input);
    let t = T::deserialize(&mut deserializer)?;
    deserializer.input = deserializer.input.trim();
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        log!(
            LogLevel::Warn,
            "Trailing characters: {:?}",
            deserializer.input.chars()
        );
        Err(Error::TrailingCharacters)
    }
}

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    // Look at the first character in the input without consuming it.
    fn peek_char(&mut self) -> Result<char> {
        self.input.chars().next().ok_or(Error::Eof)
    }
    // Look for the first character that is not a whitespace in the input.
    fn peek_real_char(&mut self) -> Result<char> {
        let mut temp_input = self.input;
        let mut ch = temp_input.chars().next().ok_or(Error::Eof)?;
        while ch.is_whitespace() {
            temp_input = &temp_input[ch.len_utf8()..];
            ch = temp_input.chars().next().ok_or(Error::Eof)?;
        }
        Ok(ch)
    }

    // Consume the first character in the input.
    fn next_char(&mut self) -> Result<char> {
        let ch = self.peek_char()?;
        self.input = &self.input[ch.len_utf8()..];
        Ok(ch)
    }
    // Look for the first character that is not a whitespace in the input.
    fn next_real_char(&mut self) -> Result<char> {
        let mut ch = self.next_char()?;
        while ch.is_whitespace() {
            ch = self.next_char()?;
        }
        Ok(ch)
    }

    fn peek_str(&mut self) -> Result<String> {
        log!(LogLevel::Debug, "Peek str");
        if !self.peek_expect_char('"')? {
            return Err(Error::ExpectedString);
        }

        let start_index = self.input.find('"').ok_or(Error::Eof)? + 1;
        let next_quote = self.input[start_index..].find('"').ok_or(Error::Eof)?;
        let next_str = &self.input[start_index..start_index + next_quote];
        Ok(next_str.to_string())
    }

    fn peek_expect_str(&mut self, expected: &str) -> Result<bool> {
        log!(LogLevel::Debug, "Peek expect str");
        let parsed_str = self.peek_str()?;
        if parsed_str != expected {
            log!(
                LogLevel::Debug,
                "Expected: {:?}, got: {:?}\n{:?}",
                expected,
                parsed_str,
                self.input.chars()
            );
            return Ok(false);
        }
        Ok(true)
    }
    fn peek_expect_char(&mut self, expected: char) -> Result<bool> {
        log!(LogLevel::Debug, "Peek expect char");
        let ch = self.peek_real_char()?;
        if ch != expected {
            log!(
                LogLevel::Debug,
                "Expected: {:?}, got: {:?}\n{:?}",
                expected,
                ch,
                self.input.chars()
            );
            return Ok(false);
        }
        Ok(true)
    }
    fn next_expect_char(&mut self, expected: char) -> Result<bool> {
        let ch = self.next_real_char()?;
        if ch != expected {
            log!(
                LogLevel::Debug,
                "Expected: {:?}, got: {:?}\n{:?}",
                expected,
                ch,
                self.input.chars()
            );
            return Ok(false);
        }
        Ok(true)
    }
    fn next_expect_string(&mut self, expected: &str) -> Result<bool> {
        let parsed_str = self.parse_string()?;
        if parsed_str != expected {
            log!(
                LogLevel::Debug,
                "Expected: {:?}, got: {:?}\n{:?}",
                expected,
                parsed_str,
                self.input.chars()
            );
            return Ok(false);
        }
        Ok(true)
    }

    // Parse the JSON identifier `true` or `false`.
    fn parse_bool(&mut self) -> Result<bool> {
        let input = match self.peek_real_char()? {
            '"' => {
                let parsed_str = self.parse_string()?;
                match parsed_str.to_lowercase().as_str() {
                    "false" | "no" | "0" | "true" | "yes" | "1" => Ok(parsed_str.to_lowercase()),
                    _ => Err(Error::ExpectedBoolean),
                }
            }
            _ => match self.input.to_lowercase().as_str() {
                "false" | "no" | "0" | "true" | "yes" | "1" => Ok(self.input.to_lowercase()),
                _ => Err(Error::ExpectedBoolean),
            },
        }?;

        self.input = &self.input[input.len()..];

        match input.as_str() {
            "false" | "no" | "0" => Ok(false),
            "true" | "yes" | "1" => Ok(true),
            _ => Err(Error::ExpectedBoolean),
        }
    }

    // Parse a integer from a string.
    fn parse_integer<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        log!(LogLevel::Debug, "Parse integer");
        let str = if self.input.contains('"') {
            // Handle the normal parsing through the VDF format.
            self.parse_string()?
        } else {
            // Handle one off cases where the integer is passed directly to the deserializer.
            let s = self.input;
            if s.parse::<T>().is_err() {
                return Err(Error::ExpectedInteger);
            }
            self.input = "";
            s
        };
        str.parse::<T>().map_err(|_| Error::ExpectedInteger)
    }

    // Parse a group of decimal digits as an unsigned integer of type T.
    //
    // This implementation is a bit too lenient, for example `001` is not
    // allowed in JSON. Also the various arithmetic operations can overflow and
    // panic or return bogus data. But it is good enough for example code!
    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8> + FromStr,
    {
        self.parse_integer::<T>()
    }

    // Parse a possible minus sign followed by a group of decimal digits as a
    // signed integer of type T.
    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8> + FromStr,
    {
        self.parse_integer::<T>()
    }

    // Parse a string until the next '"' character.
    //
    // Makes no attempt to handle escape sequences. What did you expect? This is
    // example code!
    fn parse_string(&mut self) -> Result<&'de str> {
        if !self.next_expect_char('"')? {
            return Err(Error::ExpectedString);
        }
        match self.input.find('"') {
            Some(len) => {
                let s = &self.input[..len];
                self.input = &self.input[len + 1..];
                log!(LogLevel::Debug, "Parse string: {:?}", s);
                Ok(s)
            }
            None => Err(Error::Eof),
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek_ch = self.peek_real_char()?;
        log!(LogLevel::Debug, "Deserialize any = {:?}", peek_ch);
        match peek_ch {
            'n' => self.deserialize_unit(visitor),
            't' | 'f' => self.deserialize_bool(visitor),
            '"' => self.deserialize_str(visitor),
            '0'..='9' => self.deserialize_u64(visitor),
            '-' => self.deserialize_i64(visitor),
            '[' => self.deserialize_seq(visitor),
            '{' => self.deserialize_map(visitor),
            _ => {
                log!(LogLevel::Warn, "Deserialize any = {:?}", peek_ch);
                Err(Error::Syntax)
            }
        }
    }

    // Uses the `parse_bool` parsing function defined above to read the JSON
    // identifier `true` or `false` from the input.
    //
    // Parsing refers to looking at the input and deciding that it contains the
    // JSON value `true` or `false`.
    //
    // Deserialization refers to mapping that JSON value into Serde's data
    // model by invoking one of the `Visitor` methods. In the case of JSON and
    // bool that mapping is straightforward so the distinction may seem silly,
    // but in other cases Deserializers sometimes perform non-obvious mappings.
    // For example the TOML format has a Datetime type and Serde's data model
    // does not. In the `toml` crate, a Datetime in the input is deserialized by
    // mapping it to a Serde data model "struct" type with a special name and a
    // single field containing the Datetime represented as a string.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_bool()?;
        log!(LogLevel::Debug, "Deserialize bool = {:?}", value);
        visitor.visit_bool(value)
    }

    // The `parse_signed` function is generic over the integer type `T` so here
    // it is invoked with `T=i8`. The next 8 methods are similar.
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_signed()?;
        log!(LogLevel::Debug, "Deserialize i8 = {:?}", value);
        visitor.visit_i8(value)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_signed()?;
        log!(LogLevel::Debug, "Deserialize i16 = {:?}", value);
        visitor.visit_i16(value)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_signed()?;
        log!(LogLevel::Debug, "Deserialize i32 = {:?}", value);
        visitor.visit_i32(value)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_signed()?;
        log!(LogLevel::Debug, "Deserialize i64 = {:?}", value);
        visitor.visit_i64(value)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_unsigned()?;
        log!(LogLevel::Debug, "Deserialize u8 = {:?}", value);
        visitor.visit_u8(value)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_unsigned()?;
        log!(LogLevel::Debug, "Deserialize u16 = {:?}", value);
        visitor.visit_u16(value)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_unsigned()?;
        log!(LogLevel::Debug, "Deserialize u32 = {:?}", value);
        visitor.visit_u32(value)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_unsigned()?;
        log!(LogLevel::Debug, "Deserialize u64 = {:?}", value);
        visitor.visit_u64(value)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize f32");
        let value = self.parse_integer()?;
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize f64");
        let value = self.parse_integer()?;
        visitor.visit_f64(value)
    }

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let ch = self.parse_string()?.chars().next().ok_or(Error::Eof)?;
        log!(LogLevel::Debug, "Deserialize char = {:?}", ch);
        _visitor.visit_char(ch)
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let str = self.parse_string()?;
        log!(LogLevel::Debug, "Deserialize str = {:?}", str);

        self.array_key = Some(str.to_string());

        visitor.visit_borrowed_str(str)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize string");
        self.deserialize_str(visitor)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize bytes");
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize byte buf");
        unimplemented!()
    }

    // An absent optional is represented as the JSON `null` and a present
    // optional is represented as just the contained value.
    //
    // As commented in `Serializer` implementation, this is a lossy
    // representation. For example the values `Some(())` and `None` both
    // serialize as just `null`. Unfortunately this is typically what people
    // expect when working with JSON. Other formats are encouraged to behave
    // more intelligently if possible.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize option");
        if self.input.starts_with("null") {
            self.input = &self.input["null".len()..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize unit");
        if self.input.starts_with("null") {
            self.input = &self.input["null".len()..];
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull)
        }
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize unit struct");
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize seq");
        adjust_indentation!(1);
        if let Some(key) = &self.array_key {
            let value = visitor.visit_seq(VDFSeq::new(self, key.clone()))?;
            adjust_indentation!(-1);
            log!(LogLevel::Debug, "Deserialize seq end");
            return Ok(value);
        }

        Err(Error::ExpectedMapKey)
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize tuple");
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize tuple struct");
        self.deserialize_seq(visitor)
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(
            LogLevel::Debug,
            "Deserialize map start - {:?}\n{}",
            self.beginning,
            self.input
        );

        self.beginning = false;

        // Parse the opening brace of the map.
        if self.next_expect_char('{')? {
            adjust_indentation!(1);
            // Give the visitor access to each entry of the map.
            let value = visitor.visit_map(VDFMap::new(self))?;
            // Parse the closing brace of the map.
            if self.next_expect_char('}')? {
                adjust_indentation!(-1);
                log!(LogLevel::Debug, "Deserialize map end");
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    // Structs look just like maps in JSON.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(
            LogLevel::Debug,
            "Deserialize struct start - {:?}",
            self.beginning
        );

        self.beginning = false;

        let result = self.deserialize_map(visitor);

        log!(LogLevel::Debug, "Deserialize struct end");

        result
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(LogLevel::Debug, "Deserialize enum");

        if self.beginning && peek_expect_char(&self.input, 0, '{').unwrap_or(false) {
            // Trim the first and last characters out.
            self.input = &self.input[1..self.input.len() - 2];
        }

        self.beginning = false;

        let value = parse_string(&self.input);
        if let Ok((parsed_str, _)) = value {
            println!(
                "{:?} - {:?}",
                self.input.chars(),
                peek_real_char(self.input, 0)
            );
            if peek_expect_char(self.input, 0, '{').unwrap_or(false)
                || peek_expect_char(self.input, 0, '"').unwrap_or(false)
            {
                log!(
                    LogLevel::Debug,
                    "Deserializing - Enum NewType/Tuple/Struct variant"
                );
                // Visit a newtype variant, tuple variant, or struct variant.
                let value = visitor.visit_enum(Enum::new(self))?;
                Ok(value)
            } else {
                // Visit a unit variant.
                log!(LogLevel::Debug, "Deserializing - Enum Unit variant");
                visitor.visit_enum(parsed_str.into_deserializer())
            }
        } else {
            Err(Error::ExpectedEnum)
        }
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In JSON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        log!(
            LogLevel::Debug,
            "Deserialize identifier: {:?}",
            self.peek_str()
        );
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // panic!("deserialize_ignored_any not supported");
        Err(Error::UnsupportedSelfDiscribing)
    }
}

// In order to handle commas correctly when deserializing a JSON array or map,
// we need to track whether we are on the first element or past the first
// element.
struct VDFSeq<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
    key: String,
}

impl<'a, 'de> VDFSeq<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, key: String) -> Self {
        Self {
            de,
            first: true,
            key,
        }
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for VDFSeq<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        log!(LogLevel::Debug, "--- Seq Access Element ---");
        log!(LogLevel::Debug, "Next element seed");
        if self.de.peek_real_char()? == '}' {
            // End of the map.
            log!(LogLevel::Debug, "End of Seq");
            log!(LogLevel::Debug, "--- Seq Access Element End ---");
            return Ok(None);
        }
        if !self.first && !self.de.peek_expect_str(&self.key)? {
            log!(LogLevel::Debug, "End of Seq - {:?}", self.key);
            log!(LogLevel::Debug, "--- Seq Access Element End ---");
            return Ok(None);
        }

        // Strip the key out.
        log!(
            LogLevel::Debug,
            "Stripping key: {:?} - {}",
            self.key,
            !self.first
        );
        if !self.first && !self.de.next_expect_string(&self.key)? {
            return Err(Error::ExpectedString);
        }

        self.first = false;

        // Deserialize an array element.
        let result = seed.deserialize(&mut *self.de).map(Some);
        log!(LogLevel::Debug, "--- Seq Access Element End ---");
        result
    }
}

// In order to handle commas correctly when deserializing a JSON array or map,
// we need to track whether we are on the first element or past the first
// element.
struct VDFMap<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> VDFMap<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de, first: true }
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for VDFMap<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Check if there are no more entries.
        if self.de.peek_real_char()? == '}' {
            return Ok(None);
        }
        log!(LogLevel::Debug, "Next key seed");
        // Comma is required before every entry except the first.
        self.first = false;
        // Deserialize a map key.
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        log!(LogLevel::Debug, "Next value seed");
        // Deserialize a map value.
        seed.deserialize(&mut *self.de)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // Err(Error::UnsupportedEnums)
        log!(LogLevel::Debug, "Variant seed");
        // The `deserialize_enum` method parsed a `{` character so we are
        // currently inside of a map. The seed will be deserializing itself from
        // the key of the map.
        let val = seed.deserialize(&mut *self.de)?;
        // Parse the colon separating map key from value.
        Ok((val, self))
        // if self.de.next_expect_char(':')? {
        //     Ok((val, self))
        // } else {
        //     Err(Error::ExpectedMapColon)
        // }
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        log!(LogLevel::Error, "Unit variant");
        Err(Error::UnsupportedEnums)
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    mod enums {
        use super::*;

        #[derive(Deserialize, PartialEq, Debug)]
        enum UnitEnum {
            A,
            B,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        enum NewtypeEnum {
            A(u32),
            B(u32),
        }

        #[derive(Deserialize, PartialEq, Debug)]
        enum TupleEnum {
            A(u32, u32),
            B(u32, u32),
        }

        #[derive(Deserialize, PartialEq, Debug)]
        enum StructEnum {
            A { a: u32, b: u32 },
            B { a: u32, b: u32 },
        }

        #[test]
        fn unsupported_unit_enum() {
            let expected_str = r#""A""#;
            let result = from_str::<UnitEnum>(expected_str).is_err();
            assert!(result);
        }

        #[test]
        fn supported_newtype_enum() {
            let expected_str = r#""A" "1""#;
            let expected = NewtypeEnum::A(1);
            let result = from_str::<NewtypeEnum>(expected_str).unwrap();
            assert_eq!(expected, result);
        }

        #[test]
        fn supported_tuple_enum() {
            let expected_str = r#""A" "1" "A" "2""#;
            let expected = TupleEnum::A(1, 2);
            let result = from_str::<TupleEnum>(expected_str).unwrap();
            log!(LogLevel::Debug, "{:?}", result);
            assert_eq!(expected, result);
        }

        #[test]
        fn supported_struct_enum() {
            let expected_str = r#""A" { "a" "1" "b" "2" }"#;
            let expected = StructEnum::A { a: 1, b: 2 };
            let result = from_str::<StructEnum>(expected_str).unwrap();
            log!(LogLevel::Debug, "{:?}", result);
            assert_eq!(expected, result);
        }
    }

    mod primitives {
        use super::*;

        #[test]
        fn supported_string() {
            let expected_str = r#""Hello, World!""#;
            let expected = "Hello, World!";
            let result = from_str::<String>(expected_str).unwrap();
            assert_eq!(expected, result);
        }

        #[test]
        fn support_char() {
            let expected_str = r#""a""#;
            let expected = 'a';
            let result = from_str::<char>(expected_str).unwrap();
            assert_eq!(expected, result);
        }

        #[test]
        fn supported_bool_true() {
            assert_eq!(true, from_str("1").unwrap());
        }

        #[test]
        fn supported_bool_false() {
            assert_eq!(false, from_str("0").unwrap());
        }

        #[test]
        fn test_f32() {
            let input: f32 = from_str("127.24").unwrap();
            assert_eq!(127.24f32, input);
        }

        #[test]
        fn test_f64() {
            let input: f64 = from_str("123.24").unwrap();
            assert_eq!(123.24f64, input);
        }

        #[test]
        fn test_i8() {
            let input: i8 = from_str("127").unwrap();
            assert_eq!(127, input);
        }

        #[test]
        fn test_i16() {
            let input: i16 = from_str("32767").unwrap();
            assert_eq!(32767, input);
        }

        #[test]
        fn test_i32() {
            let input: i32 = from_str("2147483647").unwrap();
            assert_eq!(2147483647, input);
        }

        #[test]
        fn test_i64() {
            let input: i64 = from_str("9223372036854775807").unwrap();
            assert_eq!(9223372036854775807, input);
        }

        #[test]
        fn test_u8() {
            let input: u8 = from_str("255").unwrap();
            assert_eq!(255, input);
        }

        #[test]
        fn test_u16() {
            let input: u16 = from_str("65535").unwrap();
            assert_eq!(65535, input);
        }

        #[test]
        fn test_u32() {
            let input: u32 = from_str("4294967295").unwrap();
            assert_eq!(4294967295, input);
        }

        #[test]
        fn test_u64() {
            let input: u64 = from_str("18446744073709551615").unwrap();
            assert_eq!(18446744073709551615, input);
        }
    }

    mod structs {
        use super::*;

        #[derive(Deserialize, PartialEq, Debug)]
        struct TestItem {
            name: String,
            value: u32,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct TestNestedData {
            int: u32,
            seq: Vec<TestItem>,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct TestData {
            int: u32,
            seq: Vec<String>,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct NestedContainer {
            test: TestNestedData,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct Container {
            test: TestData,
        }

        #[test]
        fn test_struct_perfect_order() {
            println!("Test 1 - Perfectly Ordered Data Set.");
            let expected_str = r#"
            "test"
            {
                "int"       "1"
                "seq"       "a"
                "seq"       "b"
            }"#;
            let expected = Container {
                test: TestData {
                    int: 1,
                    seq: vec!["a".to_owned(), "b".to_owned()],
                },
            };
            let result = from_str(expected_str).unwrap();
            log!(LogLevel::Debug, "{:?}", result);
            assert_eq!(expected, result);
        }

        #[test]
        fn test_struct_unordered() {
            println!("Test 2 - Unordered Data Set.");
            let expected_str = r#"
            "test"
            {
                "seq"    "a"
                "seq"    "b"
                "int"    "1"
            }"#;
            let expected = Container {
                test: TestData {
                    int: 1,
                    seq: vec!["a".to_owned(), "b".to_owned()],
                },
            };
            let result = from_str(expected_str).unwrap();
            log!(LogLevel::Debug, "{:?}", result);
            assert_eq!(expected, result);
        }

        #[test]
        fn test_struct_mixed() {
            println!("Test 3 - Mixed Data Set.");
            let expected_str = r#"
            "test"
            {
                "seq"    "a"
                "int"    "1"
                "seq"    "b"
            }"#;
            let expected = Container {
                test: TestData {
                    int: 1,
                    seq: vec!["a".to_owned(), "b".to_owned()],
                },
            };
            let result = from_str(expected_str).unwrap();
            log!(LogLevel::Debug, "{:?}", result);
            assert_eq!(expected, result);
        }

        #[test]
        fn test_nested_struct_perfect_order() {
            println!("Test 1 - Perfectly Ordered Data Set.");
            let expected_str = r#"
            "test"
            {
                "int"    "1"
                "seq"
                {
                    "name"    "a"
                    "value"    "1"
                }
                "seq"
                {
                    "name"    "b"
                    "value"    "2"
                }
            }"#;
            let expected = NestedContainer {
                test: TestNestedData {
                    int: 1,
                    seq: vec![
                        TestItem {
                            name: "a".to_owned(),
                            value: 1,
                        },
                        TestItem {
                            name: "b".to_owned(),
                            value: 2,
                        },
                    ],
                },
            };
            let result: NestedContainer = from_str(expected_str).unwrap();
            log!(LogLevel::Debug, "{:?}", result);
            assert_eq!(expected, result);
        }

        #[test]
        fn test_nested_struct_unordered() {
            println!("Test 2 - Unordered Data Set.");
            let expected_str = r#"
            "test"
            {
                "seq"
                {
                    "name"    "a"
                    "value"    "1"
                }
                "seq"
                {
                    "name"    "b"
                    "value"    "2"
                }
                "int"    "1"
            }"#;
            let expected = NestedContainer {
                test: TestNestedData {
                    int: 1,
                    seq: vec![
                        TestItem {
                            name: "a".to_owned(),
                            value: 1,
                        },
                        TestItem {
                            name: "b".to_owned(),
                            value: 2,
                        },
                    ],
                },
            };
            let result = from_str(expected_str).unwrap();
            log!(LogLevel::Debug, "{:?}", result);
            assert_eq!(expected, result);
        }
    }
}
