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
    <meta charset="UTF-8">
    <title>MASD</title>

    <style>
        html, body {
            margin: 0;
            padding: 0;
            background: black;
            overflow: hidden;
        }

        #screen {
            width: 1024px;
            height: 768px;
            display: block;
        }

        #stats {
            position: fixed;
            top: 10px;
            left: 10px;
            padding: 6px 10px;
            color: white;
            background: rgba(0, 0, 0, 0.7);
            font-family: monospace;
            font-size: 14px;
            z-index: 10;
        }
    </style>
</head>

<body>

<div id="stats">
    FPS: 0
</div>

<img id="screen">

<script>
const socket = new WebSocket("ws://" + location.host + "/ws");

const image = document.getElementById("screen");
const stats = document.getElementById("stats");

let latestFrame = null;
let displaying = false;

let frames = 0;
let lastFpsTime = performance.now();

socket.binaryType = "arraybuffer";

socket.onopen = () => {
    console.log("Connected");
};

socket.onmessage = (event) => {
    /*
     * Only keep the newest frame.
     *
     * If the browser is still displaying an old frame,
     * newer frames replace the previous pending frame.
     */
    latestFrame = event.data;

    if (!displaying) {
        displayNext();
    }
};

function displayNext() {
    if (latestFrame === null) {
        displaying = false;
        return;
    }

    displaying = true;

    // Take the newest frame.
    const frame = latestFrame;
    latestFrame = null;

    const blob = new Blob(
        [frame],
        { type: "image/jpeg" }
    );

    const url = URL.createObjectURL(blob);

    image.onload = () => {
        URL.revokeObjectURL(url);

        frames++;

        displayNext();
    };

    image.onerror = () => {
        URL.revokeObjectURL(url);

        displaying = false;
    };

    image.src = url;
}


// FPS counter
setInterval(() => {
    const now = performance.now();
    const elapsed = now - lastFpsTime;

    const fps = frames * 1000 / elapsed;

    stats.textContent =
        "FPS: " + fps.toFixed(1);

    frames = 0;
    lastFpsTime = now;
}, 1000);


socket.onclose = () => {
    console.log("Disconnected");
};

socket.onerror = (error) => {
    console.error("WebSocket error:", error);
};
</script>

</body>
</html>
    "#,
  )
}
