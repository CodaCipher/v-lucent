@echo off
cd /d S:\OpenClaw\OpenClawGUI\local-companion

echo Testing HTTP REST API...
echo.

echo Test 1: Emotion - Lustful
python nerve_link_test.py emotion lustful
timeout /t 2

echo.
echo Test 2: Emotion - Happy
python nerve_link_test.py emotion happy
timeout /t 2

echo.
echo Test 3: Speak
python nerve_link_test.py speak "Lilith'ten merhaba!"
timeout /t 3

echo.
echo Test 4: Emotion - Sad
python nerve_link_test.py emotion sad
timeout /t 2

echo.
echo All tests completed!
pause
