use actix_web::{get, HttpResponse, Responder};

#[get("/ping")]
async fn ping() -> impl Responder {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /ping", time_ns);
    HttpResponse::Ok().body("pong")
}