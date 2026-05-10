#!/usr/bin/env python3
"""CDP dispatch for web-based reviewers. Sends prompt, waits for Update File, clicks it."""
import asyncio, websockets, json, sys, time, urllib.request

CDP_PORT = 9222
POLL_INTERVAL = 5
TIMEOUT = 120

def find_tab(match_str):
    """Find a Chrome tab by title/URL substring, return its WS debugger URL."""
    tabs = json.loads(urllib.request.urlopen(f"http://localhost:{CDP_PORT}/json/list").read())
    lower = match_str.lower()
    for t in tabs:
        url = t.get("url", "").lower()
        title = t.get("title", "").lower()
        if (lower in url or lower in title) and "codex/cloud" not in url and t.get("type") == "page":
            return t["webSocketDebuggerUrl"]
    return None

async def send_prompt(ws_url, prompt):
    """Send prompt to ChatGPT input and press Enter."""
    async with websockets.connect(ws_url) as ws:
        # Focus and clear
        await ws.send(json.dumps({"id": 1, "method": "Runtime.evaluate", "params": {
            "expression": "(function(){var el=document.querySelector('#prompt-textarea');if(!el)el=document.querySelector('[contenteditable]');if(!el)return'no input';el.focus();el.textContent='';return'focused'})()",
            "returnByValue": True
        }}))
        await ws.recv()

        await asyncio.sleep(0.5)

        # Insert text (React-compatible)
        await ws.send(json.dumps({"id": 2, "method": "Input.insertText", "params": {"text": prompt}}))
        await ws.recv()

        await asyncio.sleep(1)

        # Press Enter
        await ws.send(json.dumps({"id": 3, "method": "Input.dispatchKeyEvent", "params": {
            "type": "keyDown", "key": "Enter", "code": "Enter",
            "windowsVirtualKeyCode": 13, "nativeVirtualKeyCode": 13
        }}))
        await ws.recv()
        await ws.send(json.dumps({"id": 4, "method": "Input.dispatchKeyEvent", "params": {
            "type": "keyUp", "key": "Enter", "code": "Enter",
            "windowsVirtualKeyCode": 13, "nativeVirtualKeyCode": 13
        }}))
        await ws.recv()

    return True

async def poll_and_click(ws_url):
    """Poll for Update File button and click it via mouse event."""
    start = time.time()
    while time.time() - start < TIMEOUT:
        try:
            async with websockets.connect(ws_url) as ws:
                await ws.send(json.dumps({"id": 1, "method": "Runtime.evaluate", "params": {
                    "expression": "(function(){for(var b of document.querySelectorAll('button')){if(b.textContent.trim()==='Update File'){var r=b.getBoundingClientRect();return JSON.stringify({x:r.x+r.width/2,y:r.y+r.height/2})}}return'w'})()",
                    "returnByValue": True
                }}))
                r = json.loads(await ws.recv())
                v = r.get("result", {}).get("result", {}).get("value", "w")

                if v != "w":
                    p = json.loads(v)
                    await ws.send(json.dumps({"id": 2, "method": "Input.dispatchMouseEvent", "params": {
                        "type": "mousePressed", "x": p["x"], "y": p["y"], "button": "left", "clickCount": 1
                    }}))
                    await ws.recv()
                    await ws.send(json.dumps({"id": 3, "method": "Input.dispatchMouseEvent", "params": {
                        "type": "mouseReleased", "x": p["x"], "y": p["y"], "button": "left", "clickCount": 1
                    }}))
                    await ws.recv()
                    return "CLICKED"
        except Exception:
            pass

        await asyncio.sleep(POLL_INTERVAL)

    return "TIMEOUT"

async def main():
    if len(sys.argv) < 3:
        print("Usage: cdp-dispatch.py <tab_match> <prompt>")
        sys.exit(1)

    tab_match = sys.argv[1]
    prompt = sys.argv[2]

    ws_url = find_tab(tab_match)
    if not ws_url:
        print("NO_TAB")
        sys.exit(1)

    await send_prompt(ws_url, prompt)
    print("PROMPT_SENT")

    result = await poll_and_click(ws_url)
    print(result)

asyncio.run(main())
