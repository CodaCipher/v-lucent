import { WebSocketServer } from "ws";

const wss = new WebSocketServer({ port: 9210 });

const emotions = ["happy", "sad", "angry", "surprised", "neutral"];
const texts = [
  "Merhaba! Bugün hava çok güzel!",
  "Bu biraz üzücü bir haber...",
  "Ne?! Bunu beklemiyordum!",
  "Hmm, düşünmem lazım.",
  "Harika bir fikir, hemen yapalım!",
];

console.log("Test WS server running on ws://localhost:9210");

wss.on("connection", (ws) => {
  console.log("Client connected");

  let i = 0;
  const interval = setInterval(() => {
    const msg = {
      channel: "chat",
      payload: {
        text: texts[i % texts.length],
        emotion: emotions[i % emotions.length],
        intensity: 0.5 + Math.random() * 0.5,
      },
    };
    ws.send(JSON.stringify(msg));
    console.log("Sent:", msg.payload.emotion, "-", msg.payload.text);
    i++;
  }, 5000);

  ws.on("close", () => {
    clearInterval(interval);
    console.log("Client disconnected");
  });
});
