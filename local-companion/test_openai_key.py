import os
import requests
import json

API_KEY = os.environ.get("OPENAI_API_KEY") or input("OPENAI_API_KEY girin: ").strip()

print("Testing OpenAI API key...")

# Test 1: Check if key is valid
response = requests.get(
    "https://api.openai.com/v1/models",
    headers={"Authorization": f"Bearer {API_KEY}"}
)

print(f"Status: {response.status_code}")
if response.status_code == 200:
    print("✓ API key is valid")
    models = response.json()
    print(f"Available models: {len(models['data'])}")
else:
    print(f"✗ API key error: {response.text}")

# Test 2: Create realtime session
print("\nTesting Realtime session creation...")
response = requests.post(
    "https://api.openai.com/v1/realtime/sessions",
    headers={
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json"
    },
    json={
        "model": "gpt-4o-realtime-preview-2024-12-26",
        "voice": "alloy",
        "max_response_output_tokens": 4096,
    }
)

print(f"Status: {response.status_code}")
if response.status_code == 200:
    data = response.json()
    print("✓ Session created successfully")
    print(f"Session ID: {data.get('id')}")
    print(f"Client secret: {data.get('client_secret', {}).get('value', 'N/A')[:20]}...")
    print(f"Expires at: {data.get('client_secret', {}).get('expires_at')}")
else:
    print(f"✗ Session creation error: {response.text}")
