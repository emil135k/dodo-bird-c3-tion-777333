#!/usr/bin/env python3
"""Poll for ChatGPT 'Update File' button and click it via CDP mouse event."""
import asyncio, websockets, json, sys, time

CDP_PORT = 9222
POLL_INTERVAL = 5
TIMEOUT = 120

async def find_chatgpt_ws():
    import urllib.request
    tabs = json.loads(urllib.request.urlopen(f"http://localhost:{CDP_PORT}/json/list").read())
    for t in tabs:
        url = t.get("url", "").lower()
        title = t.get("title", "").lower()
        if ("chatgpt" in url or "chatgpt" in title) and "codex/cloud" not in url and t.get("type") == "page":
            return t["webSocketDebuggerUrl"]
    return None

async def click_update_file():
    ws_url = await find_chatgpt_ws()
    if not ws_url:
        print("NO_TAB")
        return

    start = time.time()
    while time.time() - start < TIMEOUT:
        try:
            async with websockets.connect(ws_url) as ws:
                await ws.send(json.dumps({
                    "id": 1,
                    "method": "Runtime.evaluate",
                    "params": {
                        "expression": "(function(){var buttons=document.querySelectorAll('button');for(var b of buttons){if(b.textContent.trim()==='Update File'){var r=b.getBoundingClientRect();return JSON.stringify({x:r.x+r.width/2,y:r.y+r.height/2})}}return'w'})()",
                        "returnByValue": True
                    }
                }))
                r = json.loads(await ws.recv())
                v = r.get("result", {}).get("result", {}).get("value", "w")

                if v != "w":
                    p = json.loads(v)
                    await ws.send(json.dumps({"id": 2, "method": "Input.dispatchMouseEvent", "params": {"type": "mousePressed", "x": p["x"], "y": p["y"], "button": "left", "clickCount": 1}}))
                    await ws.recv()
                    await ws.send(json.dumps({"id": 3, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseReleased", "x": p["x"], "y": p["y"], "button": "left", "clickCount": 1}}))
                    await ws.recv()
                    print("CLICKED")
                    return
        except Exception:
            pass

        await asyncio.sleep(POLL_INTERVAL)

    print("TIMEOUT")

asyncio.run(click_update_file())
