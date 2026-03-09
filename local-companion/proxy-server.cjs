const express = require('express');
const axios = require('axios');
const app = express();

app.use(express.json());

// VPS'deki Lilith Proxy v3'e yönlendir
const OPENCLAW_URL = 'http://89.167.21.167:3031';  // VPS IP + Lilith Proxy
const GATEWAY_TOKEN = '36f96a021f9124172348ad69260f6d6ab03da31c6358b57c';  // OpenClaw Gateway Token
const PROXY_PORT = 3032;  // Windows proxy

// POST /api/command — Companion App'tan gelen mesajları OpenClaw'a gönder
app.post('/api/command', async (req, res) => {
  try {
    const { message, sender } = req.body;
    
    console.log(`[Proxy] Received message from ${sender}: "${message}"`);
    
    // OpenClaw'ın sessions_send API'sini kullan
    // Veya doğrudan /api/command'e POST yap (eğer varsa)
    
    // Doğrudan VPS Lilith Proxy'ye /api/command ile POST yap
    let openclaw_response;
    try {
      console.log('[Proxy] Sending to Lilith Proxy at', OPENCLAW_URL);
      openclaw_response = await axios.post(`${OPENCLAW_URL}/api/command`, {
        message: message,
        sender: sender || 'WebUI'
      }, {
        headers: {
          'Authorization': `Bearer ${GATEWAY_TOKEN}`,
          'Content-Type': 'application/json'
        }
      });
      console.log('[Proxy] ✓ Response received from Lilith Proxy');
    } catch (error) {
      console.log('[Proxy] ✗ Connection failed:');
      console.log('  Status:', error.response?.status);
      console.log('  Message:', error.message);
      console.log('  Data:', error.response?.data);
      throw error;
    }
    
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
