const express = require('express');
const axios = require('axios');
const app = express();

app.use(express.json());

const OPENCLAW_URL = 'http://localhost:18789';
const PROXY_PORT = 3031;

// POST /api/command — Companion App'tan gelen mesajları OpenClaw'a gönder
app.post('/api/command', async (req, res) => {
  try {
    const { message, sender } = req.body;
    
    console.log(`[Proxy] Received message from ${sender}: "${message}"`);
    
    // OpenClaw'ın sessions_send API'sini kullan
    // Veya doğrudan /api/command'e POST yap (eğer varsa)
    
    // Option 1: sessions_send API (OpenClaw native)
    const openclaw_response = await axios.post(`${OPENCLAW_URL}/sessions_send`, {
      message: message,
      sender: sender || 'WebUI'
    }).catch(async (error) => {
      // Fallback: Eğer sessions_send yoksa, /api/command'e dene
      console.log('[Proxy] sessions_send failed, trying /api/command...');
      return axios.post(`${OPENCLAW_URL}/api/command`, {
        message: message,
        sender: sender || 'WebUI'
      });
    });
    
    const data = openclaw_response.data;
    console.log('[Proxy] OpenClaw response:', data);
    
    // Response'u Companion App format'ına dönüştür
    const response_data = {
      text: data.text || data.message || data.response || 'Cevap alınamadı',
      emotion: data.emotion || 'neutral',
      original: data // Debug için original response'u da gönder
    };
    
    console.log('[Proxy] Formatted response:', response_data);
    res.json(response_data);
    
  } catch (error) {
    console.error('[Proxy] Error:', error.message);
    
    // Fallback response
    res.status(500).json({
      text: `Hata: ${error.message}`,
      emotion: 'sad',
      error: error.message
    });
  }
});

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'ok', message: 'Proxy is running' });
});

app.listen(PROXY_PORT, '127.0.0.1', () => {
  console.log(`[Proxy] Server listening on http://127.0.0.1:${PROXY_PORT}`);
  console.log(`[Proxy] Forwarding to OpenClaw at ${OPENCLAW_URL}`);
});
