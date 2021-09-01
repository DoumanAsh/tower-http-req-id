# tower-http-req-id

[![Crates.io](https://img.shields.io/crates/v/tower-http-req-id.svg)](https://crates.io/crates/tower-http-req-id)
[![Documentation](https://docs.rs/tower-http-req-id/badge.svg)](https://docs.rs/crate/tower-http-req-id/)
[![Build](https://github.com/DoumanAsh/tower-http-req-id/workflows/Rust/badge.svg)](https://github.com/DoumanAsh/tower-http-req-id/actions?query=workflow%3ARust)

Tower middleware to generate and make use of request id.

This middleware checks if request has `x-request-id` set by user and adds it the extension.
Note that if header's value is not valid unicode string, then it is considered non-existing.
If it is not present or invalid value for this type of ID, then automatically generates using specified generator.

To cover as many strategies as possible, it is best to use `String` type that can accept any type of id from client.

## Features:

- `uuid` - Enables UUID based generator.

## Defining own ID generator:

```rust
use tower_http_req_id::{IdGen, GenerateRequestIdLayer};

//Clone trait is necessary to fit tower's bounds
#[derive(Clone)]
struct TestGenerator;

impl IdGen<String> for TestGenerator {
    #[inline(always)]
    fn gen(&self) -> String {
        "whatever".to_owned()
    }
}

//First type parameter is generator, which can be determined from `new` function.
//Second type parameter is ID's type, which must be always specified manually.
let generator = GenerateRequestIdLayer::<_, String>::new(TestGenerator);
```

## Accessing ID:

ID is stored within `request` extensions map, which can be accessed by id's type.
