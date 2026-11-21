"""Local HTTPS Bitbucket Cloud contract used by the public workctl CLI tests."""
import http.server
import json
import pathlib
import re
import socket
import ssl
import sys
import urllib.parse

root = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
origin = "https://api.bitbucket.org"
base = "/2.0/repositories/"
me = {"uuid": "{aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa}"}
other = {"uuid": "{bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb}"}
approved = {2}
truncated_page = False
log = (root / "http.jsonl").open("w", buffering=1)


def author(pr, detail=False):
    account = {1: "first-member", 2: "first-member", 3: "first-member",
               6: "second-member", 7: "first-member", 8: "invited-member",
               11: "first-member" if not detail else "outside", 12: None}.get(pr, "outside")
    return dict(other, account_id=account)


def candidate(pr, state="OPEN", reviewers=None):
    result = dict(id=pr, state=state, reviewers=[me] if reviewers is None else reviewers)
    if mode == "author_team":
        result["author"] = author(pr)
    return result


class Api(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def respond(self, code, body):
        payload = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        self.route()

    def do_POST(self):
        self.route()

    def route(self):
        global truncated_page
        if self.headers.get("Authorization") != "Bearer AUTH_SECRET_MUST_NEVER_APPEAR":
            return self.respond(401, {})
        url = urllib.parse.urlsplit(self.path)
        path, query = url.path, urllib.parse.parse_qs(url.query)
        log.write(json.dumps([self.command, path]) + "\n")
        if path == "/2.0/user" and self.command == "GET":
            return self.respond(200, me)
        route = re.fullmatch(base + r"team/(one|two|unlisted|bad)/pullrequests(?:/(\d+)(/activity|/approve)?)?", path)
        if not route:
            return self.respond(404, {})
        repo, pr, suffix = route.groups()
        if pr is None and self.command == "GET":
            if query.get("state") != ["OPEN"]:
                return self.respond(400, {})
            if "page" not in query and query.get("q") != ['reviewers.uuid="' + me["uuid"] + '"']:
                return self.respond(400, {})
            if mode == "author_team" and "page" not in query:
                if "values.author.account_id" not in query.get("fields", [""])[0].split(","):
                    return self.respond(400, {})
            if repo == "unlisted":
                return self.respond(200, {"values": [candidate(10)]})
            if repo == "bad":
                return self.respond(403, {})
            if repo == "two":
                return self.respond(200, {"values": [candidate(8)]})
            if query.get("page") == ["2"]:
                if mode == "read_drop" and not truncated_page:
                    truncated_page = True
                    self.send_response(200)
                    self.send_header("Content-Length", "200")
                    self.end_headers()
                    self.wfile.write(b'{"values":[')
                    return
                # Duplicate across pages and more than the native bb CLI's 50 entries.
                values = [candidate(1), candidate(6), candidate(7)]
                if mode == "author_team":
                    values += [candidate(11), candidate(12)]
                return self.respond(200, {"values": values})
            values = [candidate(1), candidate(2), candidate(3), candidate(4, "MERGED"),
                      candidate(5, reviewers=[other]), candidate(9)]
            values += [candidate(i, reviewers=[]) for i in range(50, 94)]
            pagination = origin + path + "?state=OPEN&page=2"
            if mode == "escape":
                pagination = origin + base + "team/unlisted/pullrequests?state=OPEN&page=2"
            if mode == "cycle":
                pagination = origin + path + "?" + url.query
            return self.respond(200, {"values": values, "next": pagination})
        pr = int(pr)
        if suffix == "/activity" and self.command == "GET":
            if pr == 3:
                if query.get("page") == ["2"]:
                    return self.respond(200, {"values": [{"approval": {"user": me}}]})
                return self.respond(200, {"values": [{"approval": {"user": other}}],
                                          "next": origin + path + "?page=2"})
            if pr == 8 and pr in approved:
                return self.respond(200, {"values": [{"approval": {"user": me}}]})
            if mode == "partial" and pr == 6:
                return self.respond(503, {})
            return self.respond(200, {"values": [{"update": {"state": "OPEN"}}]})
        if suffix == "/approve" and self.command == "POST":
            if pr in approved or pr not in {1, 3, 6, 8, 11}:
                return self.respond(409, {})
            if mode == "partial" and pr == 1:
                return self.respond(503, {"message": "AUTH_SECRET_MUST_NEVER_APPEAR"})
            approved.add(pr)
            if pr == 6:
                # Service committed the approval, then the response was lost.
                self.connection.shutdown(socket.SHUT_RDWR)
                return
            return self.respond(200, {"user": me, "approved": True})
        if suffix is None and self.command == "GET":
            body = candidate(pr, "DECLINED" if pr == 9 else "OPEN",
                             [other] if pr == 7 else [me])
            participant = {"user": me, "approved": pr in approved}
            return self.respond(200, dict(body, author=author(pr, True) if mode == "author_team" else other,
                                          participants=[participant]))
        return self.respond(405, {})


with http.server.HTTPServer(("127.0.0.1", 0), Api) as server:
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(root / "cert.pem", root / "key.pem")
    server.socket = ctx.wrap_socket(server.socket, server_side=True)
    print(server.server_port, flush=True)
    server.serve_forever()
