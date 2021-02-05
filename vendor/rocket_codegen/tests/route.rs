// Rocket sometimes generates mangled identifiers that activate the
// non_snake_case lint. We deny the lint in this test to ensure that
// code generation uses #[allow(non_snake_case)] in the appropriate places.
#![deny(non_snake_case)]

#[macro_use] extern crate rocket;

use std::path::PathBuf;

use rocket::http::ext::Normalize;
use rocket::local::blocking::Client;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket::request::{Request, Form};
use rocket::http::{Status, RawStr, ContentType};

// Use all of the code generation available at once.

#[derive(FromForm, UriDisplayQuery)]
struct Inner<'r> {
    field: &'r RawStr
}

struct Simple(String);

#[async_trait]
impl FromData for Simple {
    type Error = ();

    async fn from_data(_: &Request<'_>, data: Data) -> data::Outcome<Self, ()> {
        let string = data.open(64.bytes()).stream_to_string().await.unwrap();
        data::Outcome::Success(Simple(string))
    }
}

#[post("/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>", format = "json", data = "<simple>", rank = 138)]
fn post1(
    sky: usize,
    name: &RawStr,
    a: String,
    query: Form<Inner<'_>>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.normalized_str(), simple.0);

    let uri = uri!(post2: a, name.url_decode_lossy(), path, sky, query.into_inner());

    format!("({}) ({})", string, uri.to_string())
}

#[route(POST, path = "/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>", format = "json", data = "<simple>", rank = 138)]
fn post2(
    sky: usize,
    name: &RawStr,
    a: String,
    query: Form<Inner<'_>>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.normalized_str(), simple.0);

    let uri = uri!(post2: a, name.url_decode_lossy(), path, sky, query.into_inner());

    format!("({}) ({})", string, uri.to_string())
}

#[allow(dead_code)]
#[post("/<_unused_param>?<_unused_query>", data="<_unused_data>")]
fn test_unused_params(_unused_param: String, _unused_query: String, _unused_data: Data) {
}

#[test]
fn test_full_route() {
    let rocket = rocket::ignite()
        .mount("/1", routes![post1])
        .mount("/2", routes![post2]);

    let client = Client::tracked(rocket).unwrap();

    let a = "A%20A";
    let name = "Bob%20McDonald";
    let path = "this/path/here";
    let sky = 777;
    let query = "field=inside";
    let simple = "data internals";

    let path_part = format!("/{}/{}/name/{}", a, name, path);
    let query_part = format!("?sky={}&sky=blue&{}", sky, query);
    let uri = format!("{}{}", path_part, query_part);
    let expected_uri = format!("{}?sky=blue&sky={}&{}", path_part, sky, query);

    let response = client.post(&uri).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.post(format!("/1{}", uri)).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client
        .post(format!("/1{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name, "A A", "inside", path, simple, expected_uri));

    let response = client.post(format!("/2{}", uri)).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client
        .post(format!("/2{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name, "A A", "inside", path, simple, expected_uri));
}

mod scopes {
    mod other {
        #[get("/world")]
        pub fn world() -> &'static str {
            "Hello, world!"
        }
    }

    #[get("/hello")]
    pub fn hello() -> &'static str {
        "Hello, outside world!"
    }

    use other::world;

    fn _rocket() -> rocket::Rocket {
        rocket::ignite().mount("/", rocket::routes![hello, world])
    }
}
