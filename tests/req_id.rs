use tower_http_req_id::{IdGen, GenerateRequestIdLayer};

use http::{Request, Response};
use hyper::Body;
use core::convert::Infallible;
use tower::{service_fn, ServiceBuilder, ServiceExt};

const TEST_ID: &str = "俺のidがこんな可愛いわけがない";

#[derive(Clone)]
struct TestGenerator;

impl IdGen<String> for TestGenerator {
    #[inline(always)]
    fn gen(&self) -> String {
        TEST_ID.to_owned()
    }
}

#[tokio::test]
async fn should_insert_static_string() {
    let svc = ServiceBuilder::new().layer(GenerateRequestIdLayer::<_, String>::new(TestGenerator))
                                   .service(service_fn(|req: Request<Body>| async move {
                                       let id = req.extensions().get::<String>().expect("required-id is not inserted");
                                       Ok::<_, Infallible>(Response::new(id.to_owned()))
                                   }));

    let res = svc.oneshot(Request::new(Body::empty())).await.unwrap().into_body();
    assert_eq!(TEST_ID, res);
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn should_insert_uuid_id() {
    use tower_http_req_id::{Uuid, UuidGenerator};

    let gen = UuidGenerator::new();

    let svc = ServiceBuilder::new().layer(GenerateRequestIdLayer::<_, Uuid>::new(gen))
                                   .service(service_fn(|req: Request<Body>| async move {
                                       let id = req.extensions().get::<Uuid>().expect("required-id is not inserted");
                                       Ok::<_, Infallible>(Response::new(id.to_string()))
                                   }));

    let res = svc.oneshot(Request::new(Body::empty())).await.unwrap().into_body();
    assert!(Uuid::parse_str(&res).is_ok())
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn should_insert_uuid_as_string_id() {
    use tower_http_req_id::{Uuid, UuidGenerator};

    let gen = UuidGenerator::new();

    let svc = ServiceBuilder::new().layer(GenerateRequestIdLayer::<_, String>::new(gen))
                                   .service(service_fn(|req: Request<Body>| async move {
                                       let id = req.extensions().get::<String>().expect("required-id is not inserted");
                                       Ok::<_, Infallible>(Response::new(id.to_owned()))
                                   }));

    let res = svc.oneshot(Request::new(Body::empty())).await.unwrap().into_body();
    assert!(Uuid::parse_str(&res).is_ok())
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn should_not_insert_new_uuid_id_if_header_present() {
    use tower_http_req_id::{Uuid, UuidGenerator};

    let id = Uuid::v4();
    let gen = UuidGenerator::new();

    let svc = ServiceBuilder::new().layer(GenerateRequestIdLayer::<_, Uuid>::new(gen))
                                   .service(service_fn(|req: Request<Body>| async move {
                                       let id = req.extensions().get::<Uuid>().expect("required-id is not inserted");
                                       Ok::<_, Infallible>(Response::new(id.to_string()))
                                   }));

    let mut req = Request::new(Body::empty());
    req.headers_mut().insert("x-request-id", http::HeaderValue::from_str(&id.to_str()).unwrap());
    let res = svc.oneshot(req).await.unwrap().into_body();

    let res = Uuid::parse_str(&res).expect("to have uuid");
    assert_eq!(id, res);
}

