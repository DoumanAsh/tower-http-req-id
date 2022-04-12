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

extern crate alloc;

mod utils;

pub use http;
pub use tower_layer;
pub use tower_service;

use core::{fmt, task};
use core::pin::Pin;
use core::future::Future;
use core::marker::PhantomData;

use http::{Response, Request};
use tower_layer::Layer;
use tower_service::Service;

///Header name for Request id
pub const HEADER_NAME: &str = "x-request-id";

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
///It has following requirements:
///
///- `IdGen` must be implemented for type that generates ID.
///- `ID` can be created from string by means of `FromStr` trait.
///- `ID` should be write-able in order to store it in outgoing response.
///- `ID` should be `Clone`-able in order to be copied to write it in response header.
pub trait IdType<G: IdGen<Self>>: Sized + core::str::FromStr + fmt::Display + Clone {
}

impl<G: IdGen<T> + Sized, T: Sized + core::str::FromStr + fmt::Display + Clone> IdType<G> for T {
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

//use separate type parameter for request and response bodies.
//to make sure user is free to use whatever handler he wishes.
impl<ReqBody, ResBody, S: Service<Request<ReqBody>, Response = Response<ResBody>>, O: IdType<G> + Send + Sync + 'static, G: IdGen<O> + Clone + Send + Sync + 'static> Service<Request<ReqBody>> for GenerateRequestId<S, G, O> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFut<S::Future, O>;

    #[inline]
    fn poll_ready(&mut self, ctx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(ctx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
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

        req.extensions_mut().insert(id.clone());
        ResponseFut {
            inner: self.inner.call(req),
            id,
        }
    }
}

///Future adding request-id to list of response's headers.
pub struct ResponseFut<F, T> {
    inner: F,
    id: T
}

impl<ResBody, E, F: Future<Output = Result<Response<ResBody>, E>>, T: fmt::Display> Future for ResponseFut<F, T> {
    type Output = F::Output;

    #[inline]
    fn poll(self: Pin<&mut Self>, ctx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let this = unsafe {
            Pin::get_unchecked_mut(self)
        };
        let fut = unsafe {
            Pin::new_unchecked(&mut this.inner)
        };

        let mut resp = match Future::poll(fut, ctx) {
            task::Poll::Ready(resp) => resp?,
            task::Poll::Pending => return task::Poll::Pending,
        };

        let mut header_value = crate::utils::BytesWriter::new();
        //Retarded implementation could fail intentionally, but there is no reason for proper one to fail when writing into Vec.
        let _ = fmt::Write::write_fmt(&mut header_value, format_args!("{}", this.id));

        let header_value = header_value.freeze();
        let header_value = http::HeaderValue::from_maybe_shared(header_value).expect("Generated id is not a valid header value");
        resp.headers_mut().insert(HEADER_NAME, header_value);
        task::Poll::Ready(Ok(resp))
    }
}
