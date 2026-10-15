#!/usr/bin/env python3
"""Packet probe for the Septuvo namespace lab (N1-03). Standard library only.

listen LOGFILE READYFILE
    Receive on UDP ports 53, 67, 547 and 5000 and TCP ports 53, 853 and 5000
    (IPv4 and IPv6, all local addresses). Every payload received is appended
    to LOGFILE as one line. READYFILE is created once all sockets are bound.

send udp|tcp DST PORT TOKEN [SPORT]
    Send TOKEN to DST:PORT, optionally from source port SPORT. Prints
    "sent" or "refused:<errno name>" (the kill switch makes the kernel refuse
    locally generated packets with EPERM). Always exits 0: where the token
    arrives is what the lab checks.
"""

import errno
import ipaddress
import selectors
import socket
import sys

UDP_PORTS = (53, 67, 547, 5000)
TCP_PORTS = (53, 853, 5000)


def listen(logfile, readyfile):
    sel = selectors.DefaultSelector()
    families = [(socket.AF_INET, "0.0.0.0")]
    try:
        socket.socket(socket.AF_INET6, socket.SOCK_DGRAM).close()
        families.append((socket.AF_INET6, "::"))
    except OSError:
        pass  # kernel without IPv6: run.sh skips the IPv6 cases
    for family, any_addr in families:
        for port in UDP_PORTS:
            s = socket.socket(family, socket.SOCK_DGRAM)
            if family == socket.AF_INET6:
                s.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind((any_addr, port))
            sel.register(s, selectors.EVENT_READ, "udp")
        for port in TCP_PORTS:
            s = socket.socket(family, socket.SOCK_STREAM)
            if family == socket.AF_INET6:
                s.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind((any_addr, port))
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


def send(proto, dst, port, token, sport=None):
    family = socket.AF_INET6 if ipaddress.ip_address(dst).version == 6 else socket.AF_INET
    kind = socket.SOCK_DGRAM if proto == "udp" else socket.SOCK_STREAM
    s = socket.socket(family, kind)
    s.settimeout(1.5)
    try:
        if sport:
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind(("::" if family == socket.AF_INET6 else "0.0.0.0", int(sport)))
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
    if len(argv) == 3 and argv[0] == "listen":
        listen(argv[1], argv[2])
    elif len(argv) in (5, 6) and argv[0] == "send" and argv[1] in ("udp", "tcp"):
        send(*argv[1:])
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
