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

dns-server LOGFILE READYFILE ADDRESS NAME=IPV4...
    A deliberately MALICIOUS stand-in for a network's resolver (N3-02): on
    UDP 53 at ADDRESS it answers each A query with the IPv4 address listed
    for the name (or the first one for unknown names) and logs each queried
    name. Test code only.

resolve-connect SERVER NAME PORT TOKEN
    Ask SERVER (UDP 53) for the A record of NAME, then send TOKEN over TCP
    to the answer on PORT. Prints "answer:<ip> sent", "answer:<ip>
    refused:<reason>" or "noanswer:<reason>". Always exits 0.
"""

import errno
import ipaddress
import selectors
import socket
import struct
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


def dns_query(name, ident=0x5E57):
    q = struct.pack("!HHHHHH", ident, 0x0100, 1, 0, 0, 0)
    for label in name.rstrip(".").split("."):
        q += bytes([len(label)]) + label.encode("ascii")
    return q + b"\x00" + struct.pack("!HH", 1, 1)


def dns_name(packet, offset=12):
    labels = []
    while packet[offset]:
        n = packet[offset]
        labels.append(packet[offset + 1:offset + 1 + n].decode("ascii", "replace"))
        offset += 1 + n
    return ".".join(labels), offset + 5


def dns_server(logfile, readyfile, address, mappings):
    answers = dict(m.split("=", 1) for m in mappings)
    default = mappings[0].split("=", 1)[1]
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.bind((address, 53))
    with open(readyfile, "w", encoding="ascii"):
        pass
    with open(logfile, "a", encoding="ascii", buffering=1) as log:
        while True:
            packet, peer = s.recvfrom(512)
            try:
                name, end = dns_name(packet)
            except (IndexError, UnicodeError):
                continue
            log.write(name + "\n")
            ip = answers.get(name, default)
            reply = packet[:2] + struct.pack("!HHHHH", 0x8180, 1, 1, 0, 0) + packet[12:end]
            reply += struct.pack("!HHHLH", 0xC00C, 1, 1, 60, 4) + socket.inet_aton(ip)
            s.sendto(reply, peer)


def resolve_connect(server, name, port, token):
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.settimeout(1.5)
    try:
        s.sendto(dns_query(name), (server, 53))
        reply, _ = s.recvfrom(512)
        answer = socket.inet_ntoa(reply[-4:])
    except OSError as exc:
        print("noanswer:" + (errno.errorcode.get(exc.errno, str(exc.errno)) if exc.errno else "timeout"))
        return
    finally:
        s.close()
    print("answer:" + answer + " ", end="")
    send("tcp", answer, port, token)


def main(argv):
    if len(argv) >= 4 and argv[0] == "listen":
        listen(argv[1], argv[2], argv[3:])
    elif len(argv) in (5, 7) and argv[0] == "send" and argv[1] in ("udp", "tcp"):
        send(*argv[1:])
    elif len(argv) >= 5 and argv[0] == "dns-server":
        dns_server(argv[1], argv[2], argv[3], argv[4:])
    elif len(argv) == 5 and argv[0] == "resolve-connect":
        resolve_connect(*argv[1:])
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
