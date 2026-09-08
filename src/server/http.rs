use axum::{Router, response::Html, routing::get};

use super::ws::websocket;

pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
  let app = Router::new()
    .route("/", get(index))
    .route("/ws", get(websocket));

  let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
  println!("Server listening on http://0.0.0.0:8080");
  axum::serve(listener, app).await?;

  Ok(())
}

async fn index() -> Html<&'static str> {
  Html(
    r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>MASD</title>
    </head>

    <body>
        <h1>MASD</h1>

<script>
    const socket = new WebSocket("ws://" + location.host + "/ws");

    const image = document.createElement("img");

    image.style.width = "1024px";
    image.style.height = "768px";

    document.body.appendChild(image);

    socket.onopen = () => {
        console.log("Connected");
    };

    socket.onmessage = (event) => {
        const blob = new Blob([event.data], {
            type: "image/jpeg"
        });

        const url = URL.createObjectURL(blob);

        image.onload = () => {
            URL.revokeObjectURL(url);
        };

        image.src = url;
    };

    socket.onclose = () => {
        console.log("Disconnected");
    };
</script>
    </body>
    </html>
    "#,
  )
}
