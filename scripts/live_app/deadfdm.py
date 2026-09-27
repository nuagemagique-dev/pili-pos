# A dead FDM on 127.0.0.1:443: logs each connection attempt, closes it at once.
import socket, datetime
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", 443)); s.listen(16)
while True:
    c, _ = s.accept()
    print(datetime.datetime.now().strftime("%H:%M:%S"), "connect", flush=True)
    c.close()
