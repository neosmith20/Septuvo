#!/usr/bin/env python3
"""Packet probe for the Septuvo namespace lab (N1-03). Standard library only.

listen LOGFILE READYFILE ADDRESS...
    Receive on UDP ports 53, 67, 443, 547 and 5000 and TCP ports 53, 80, 443,
    853 and 5000,
    on each listed lab address only (never on all interfaces). Every payload
    received is appended to LOGFILE as one line. READYFILE is created once
    all sockets are bound. Addresses that appear later (the stand-in tunnel
    end) need net.ipv4.ip_nonlocal_bind / net.ipv6.ip_nonlocal_bind, which
    run.sh sets inside the throw-away namespace.

send udp|tcp DST PORT TOKEN [SPORT SRC]
    Send TOKEN to DST:PORT, optionally from source port SPORT on source
    address SRC. Prints "sent" or "refused:<errno name>" (the kill switch
    makes the kernel refuse locally generated packets with EPERM). Always
    exits 0: where the token arrives is what the lab checks.
"""

import errno
import ipaddress
import selectors
import socket
import sys

UDP_PORTS = (53, 67, 443, 547, 5000)
TCP_PORTS = (53, 80, 443, 853, 5000)


def listen(logfile, readyfile, addresses):
    sel = selectors.DefaultSelector()
    for address in addresses:
        family = socket.AF_INET6 if ipaddress.ip_address(address).version == 6 else socket.AF_INET
        for port in UDP_PORTS:
            s = socket.socket(family, socket.SOCK_DGRAM)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind((address, port))
            sel.register(s, selectors.EVENT_READ, "udp")
        for port in TCP_PORTS:
            s = socket.socket(family, socket.SOCK_STREAM)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind((address, port))
            s.listen(16)
            sel.register(s, selectors.EVENT_READ, "tcp-listen")
    with open(readyfile, "w", encoding="ascii"):
        pass
    with open(logfile, "a", encoding="ascii", buffering=1) as log:
        while True:
            for key, _ in sel.select():
                sock, kind = key.fileobj, key.data
                if kind == "udp":
                    data, _ = sock.recvfrom(256)
                    log.write(data.decode("ascii", "replace").strip() + "\n")
                elif kind == "tcp-listen":
                    conn, _ = sock.accept()
                    conn.settimeout(1)
                    try:
                        data = conn.recv(256)
                        log.write(data.decode("ascii", "replace").strip() + "\n")
                    except OSError:
                        pass
                    finally:
                        conn.close()


def send(proto, dst, port, token, sport=None, src=None):
    family = socket.AF_INET6 if ipaddress.ip_address(dst).version == 6 else socket.AF_INET
    kind = socket.SOCK_DGRAM if proto == "udp" else socket.SOCK_STREAM
    s = socket.socket(family, kind)
    s.settimeout(1.5)
    try:
        if sport:
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind((src, int(sport)))
        if proto == "udp":
            s.sendto(token.encode("ascii"), (dst, int(port)))
        else:
            s.connect((dst, int(port)))
            s.sendall(token.encode("ascii"))
        print("sent")
    except OSError as exc:
        print("refused:" + errno.errorcode.get(exc.errno, str(exc.errno)) if exc.errno else "refused:timeout")
    finally:
        s.close()


def main(argv):
    if len(argv) >= 4 and argv[0] == "listen":
        listen(argv[1], argv[2], argv[3:])
    elif len(argv) in (5, 7) and argv[0] == "send" and argv[1] in ("udp", "tcp"):
        send(*argv[1:])
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
