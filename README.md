# VDF Serde Serializer & Deserializer

A simple library for serializing and deserializing Valve Data Format (VDF) files using the Serde framework. Allowing developers to easily convert Rust data structures to and from VDF files. Enabling developers to easily read and write VDF files in Rust, in order to create useful tools and applications.

## Features
Legend:
- ✅ Supported
- ❌ Not Supported
- ⚠️ Partially Supported
- ✔ Mostly Supported but with some limitations
- 🚧 Work in Progress

| Feature                   | Serializable | Deserializable  | Notes            |
|---------------------------|--------------|-----------------|------------------|
| **Primitives**            |  ✔           |  ✔             |  |
| Primitive (Strs)          | ✅           | ✅             | Readable Strings are serialized as quoted strings. |
| Primitive (Bools)         | ✅           | ✅             | Readable Boolean as `1` or `0`. |
| Primitive (Ints)          | ✅           | ✅             | Readable Integer. |
| Primitive (Floats)        | ✅           | ✅             | Readable Float. |
| Primitive (Char)          | ✅           | ✅             | Readable Single Character. |
| Primitive (Bytes)         | ❌           | ❌             | A Single byte or a buffer of bytes. |
|                           |               |                |  |
| **Structs**               | ⚠️           | ⚠️             |  |
| Struct                    | ✅           | ✅             | Structs are serialized as key-value pairs. Try wrapping a struct inside a map / container struct |
| Struct Tuple              | ❌           | ❌             | Tuple structs are structs without field names. ``` struct TupleStruct(u32, u32) ``` |
| Struct Variant            | ✅           | ✅             | Struct variants are enums with named fields. |
|                           |               |                |  |
| **Collections**           | ✅           | ✅             |  |
| Arrays                    | ✅           | ✅             | Arrays are serialized as sequences. ``` struct STRUCT { x: Vec<u32> } ``` |
| Maps                      | ✅           | ✅             | Maps are serialized as key-value pairs. ``` struct STRUCT { x: HashMap<String, u32> } ``` |
|                           |               |                |  |
| **Enums**                 | ⚠️           | ⚠️             | Enums are serialized as key-value pairs. ``` enum ENUM {}``` |
| Enum Variant Primitive    | ✅           | ✅             | Enum newtypes are enums with a single field. ``` enum ENUM { Element(String) }``` |
| Enum Variant Struct       | ✅           | ✅             | Enum variant with a struct field. ``` enum ENUM { Element { x: u32 } } ``` |
| Enum Unit                 | ❌           | ❌             | Enum units are enums with no fields. ``` enum ENUM { Element } ``` |
| Enum Tuple Variant        | ❌           | ✅             | Enum variant with a tuple field. ``` enum ENUM { Element((u32, u32)) }``` |
| Enum Tuple Struct         | ❌           | ❌             | Enum variant with a tuple struct field. ``` struct TupleStruct(u32, u32); enum ENUM { Element(TupleStruct) } ``` |
|                           |               |                |  |
| **Tuples**                | ❌           | ⚠️             | Tuples are serialized as sequences. |
| Tuple Variant Enum        | ❌           | ✅             | Tuple variant with an enum field. ``` enum ENUM { Element(u32, u32) } ``` |
| Tuple Variant Struct      | ❌           | ❌             | Tuple variant with a struct field. ``` struct STRUCT { x: u32, y: u32 } enum ENUM { Element(u32, u32) } ``` |
|                           |               |                |  |
| **Special Types**         | ❌           | ❌             |  |
| Any                       | ❌           | ❌             | Any is a unknown type specified by the user. |
| Option                    | ❌           | ❌             | Option is a nullable type. |
| Unit                      | ❌           | ❌             | Unit is a type with no value. |

## Usage
Add this to your `Cargo.toml`:

```toml
[dependencies]
vdf_serde = "0.1.0"
```
