#!/usr/bin/env python3
"""Step-by-step WebDriver CLI for the real pili-pos binary (tauri-driver).

  wd.py start                 open a session (launches the binary)
  wd.py stop                  close it
  wd.py click CSS [TEXT]      click the first CSS match (containing TEXT)
  wd.py texts CSS             textContent of every CSS match
  wd.py wait CSS [TEXT] [S]   wait until a CSS match (containing TEXT) exists
  wd.py shot NAME             screenshot to /shots/NAME.png
  wd.py url                   current URL
  wd.py js 'return ...'       run a script
"""
import base64, json, os, sys, time, urllib.request

DRIVER = "http://127.0.0.1:4444"
BINARY = os.environ.get("PILI_POS_BINARY", "/cargo-target/release/pili-pos")
EL = "element-6066-11e4-a52e-4f735466cecf"
SESSION_FILE = "/tmp/wd-session"


def wd(method, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(DRIVER + path, data=data, method=method,
                                 headers={"content-type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            return json.loads(r.read())["value"]
    except urllib.error.HTTPError as e:
        raise SystemExit(f"{method} {path}: {e.read().decode()[:500]}")


def sid():
    return open(SESSION_FILE).read().strip()


def cmd(method, path, body=None):
    return wd(method, f"/session/{sid()}{path}", body)


def find(css, text=None):
    for ref in cmd("POST", "/elements", {"using": "css selector", "value": css}):
        eid = ref[EL]
        if text is None or text in content(eid):
            return eid
    return None


def content(eid):
    return cmd("POST", "/execute/sync", {
        "script": "return arguments[0].textContent;", "args": [{EL: eid}]}).strip()


def main(argv):
    op, args = argv[1], argv[2:]
    if op == "start":
        created = wd("POST", "/session", {"capabilities": {"alwaysMatch": {
            "tauri:options": {"application": BINARY}}}})
        open(SESSION_FILE, "w").write(created["sessionId"])
        print("session", created["sessionId"])
    elif op == "stop":
        wd("DELETE", f"/session/{sid()}")
    elif op == "click":
        eid = find(args[0], args[1] if len(args) > 1 else None)
        if not eid:
            raise SystemExit(f"no match for {args}")
        cmd("POST", f"/element/{eid}/click", {})
        print("clicked", args)
    elif op == "texts":
        refs = cmd("POST", "/elements", {"using": "css selector", "value": args[0]})
        for ref in refs:
            print(repr(content(ref[EL])))
    elif op == "wait":
        text = args[1] if len(args) > 1 and args[1] else None
        limit = float(args[2]) if len(args) > 2 else 30
        end = time.time() + limit
        while time.time() < end:
            if find(args[0], text):
                print("found", args[:2])
                return
            time.sleep(0.25)
        raise SystemExit(f"timeout waiting for {args}")
    elif op == "shot":
        png = base64.b64decode(cmd("GET", "/screenshot"))
        os.makedirs("/shots", exist_ok=True)
        open(f"/shots/{args[0]}.png", "wb").write(png)
        print(f"/shots/{args[0]}.png")
    elif op == "url":
        print(cmd("GET", "/url"))
    elif op == "js":
        print(json.dumps(cmd("POST", "/execute/sync", {"script": args[0], "args": []}),
                         ensure_ascii=False))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
