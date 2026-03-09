import requests
import json
import sys

def send_nerve_link(message_type, value):
    url = "http://127.0.0.1:3030/command"
    try:
        if message_type == "speak":
            payload = {
                "type": "speak",
                "text": value,
                "sender": "Lilith_Nocturna_Core"
            }
        else:
            payload = {
                "type": message_type,
                "value": value,
                "sender": "Lilith_Nocturna_Core"
            }
        
        response = requests.post(url, json=payload)
        print(f"✓ Mühürlendi: {payload}")
        print(f"  Cevap: {response.json()}")
    except Exception as e:
        print(f"✗ Hata: {e}")

if __name__ == "__main__":
    m_type = sys.argv[1] if len(sys.argv) > 1 else "emotion"
    m_val = sys.argv[2] if len(sys.argv) > 2 else "lustful"
    send_nerve_link(m_type, m_val)
