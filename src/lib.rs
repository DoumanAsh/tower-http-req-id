//!Tower middleware to generate and make use of request id.
//!
//!This middleware checks if request has `x-request-id` set by user and adds it the extension.
//!Note that if header's value is not valid unicode string, then it is considered non-existing.
//!If it is not present or invalid value for this type of ID, then automatically generates using specified generator.
//!
//!To cover as many strategies as possible, it is best to use `String` type that can accept any type of id from client.
//!
//!## Features:
//!
//!- `uuid` - Enables UUID based generator.
//!
//!## Defining own ID generator:
//!
//!```rust
//!use tower_http_req_id::{IdGen, GenerateRequestIdLayer};
//!
//!//Clone trait is necessary to fit tower's bounds
//!#[derive(Clone)]
//!struct TestGenerator;
//!
//!impl IdGen<String> for TestGenerator {
//!    #[inline(always)]
//!    fn gen(&self) -> String {
//!        "whatever".to_owned()
//!    }
//!}
//!
//!//First type parameter is generator, which can be determined from `new` function.
//!//Second type parameter is ID's type, which must be always specified manually.
//!let generator = GenerateRequestIdLayer::<_, String>::new(TestGenerator);
//!```
//!
//!## Accessing ID:
//!
//!ID is stored within `request` extensions map, which can be accessed by id's type.
//!


#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

pub use http;
pub use tower_layer;
pub use tower_service;

use core::task;
use core::marker::PhantomData;

use http::Request;
use tower_layer::Layer;
use tower_service::Service;

const HEADER_NAME: &str = "x-request-id";

#[cfg(feature = "uuid")]
mod uuid;
#[cfg(feature = "uuid")]
pub use uuid::{Uuid, UuidGenerator};

///Trait to generate ID
pub trait IdGen<Output>: Sized {
    ///Generate ID
    fn gen(&self) -> Output;
}

///Describes Request's ID type
///
///It has two requirements:
///
///- `IdGen` must be implemented for type that generates ID.
///- `ID` can be created from string by means of `FromStr` trait.
pub trait IdType<G: IdGen<Self>>: Sized + core::str::FromStr {
}

impl<G: IdGen<T> + Sized, T: Sized + core::str::FromStr> IdType<G> for T {
}

#[derive(Clone, Copy, Debug)]
///Layer for adding request id.
///
///See module documentation for details.
pub struct GenerateRequestIdLayer<G, O> {
    gen: G,
    _out: PhantomData<O>,
}

impl<G, O> GenerateRequestIdLayer<G, O> {
    #[inline(always)]
    ///Creates new instance
    pub const fn new(gen: G) -> Self {
        Self {
            gen,
            _out: PhantomData,
        }
    }
}

impl<G: Default, O> Default for GenerateRequestIdLayer<G, O> {
    fn default() -> Self {
        Self {
            gen: Default::default(),
            _out: PhantomData,
        }
    }
}

impl<S, G: IdGen<O> + Clone, O: IdType<G>> Layer<S> for GenerateRequestIdLayer<G, O> {
    type Service = GenerateRequestId<S, G, O>;

    #[inline(always)]
    fn layer(&self, inner: S) -> Self::Service {
        GenerateRequestId::new(inner, self.gen.clone())
    }
}

#[derive(Clone, Copy, Debug)]
///Service for adding request id.
///
///See module documentation for details.
pub struct GenerateRequestId<S, G, O> {
    inner: S,
    gen: G,
    _out: PhantomData<O>,
}

impl<S, G, O> GenerateRequestId<S, G, O> {
    #[inline(always)]
    ///Creates new instance
    pub const fn new(inner: S, gen: G) -> Self {
        Self {
            inner,
            gen,
            _out: PhantomData,
        }
    }
}

impl<ResBody, S: Service<Request<ResBody>>, O: IdType<G> + Send + Sync + 'static, G: IdGen<O> + Clone + Send + Sync + 'static> Service<Request<ResBody>> for GenerateRequestId<S, G, O> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, ctx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(ctx)
    }

    fn call(&mut self, mut req: Request<ResBody>) -> Self::Future {
        let id = match req.headers().get(HEADER_NAME) {
            Some(header) => match header.to_str() {
                Ok(header) => match O::from_str(header) {
                    Ok(id) => id,
                    Err(_) => self.gen.gen(),
                },
                Err(_) => self.gen.gen(),
            },
            None => self.gen.gen(),
        };

        req.extensions_mut().insert(id);
        self.inner.call(req)
    }
}
