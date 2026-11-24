"""Local HTTPS Bitbucket Cloud contract used by the public workctl CLI tests."""
import http.server
import json
import pathlib
import re
import socket
import ssl
import sys
import time
import threading
import urllib.parse

root = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
origin = "https://api.bitbucket.org"
base = "/2.0/repositories/"
me = {"uuid": "{aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa}"}
other = {"uuid": "{bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb}"}
approved = {2}
truncated_page = False
limits = set()
cooldown = 0
parallel_reads = threading.Barrier(2, timeout=5)
pool_advanced = threading.Event()
log = (root / "http.jsonl").open("w", buffering=1)


def author(pr, detail=False):
    account = {1: "first-member", 2: "first-member", 3: "first-member",
               6: "second-member", 7: "first-member", 8: "invited-member",
               11: "first-member" if not detail else "outside", 12: None}.get(pr, "outside")
    return dict(other, account_id=account)


def candidate(pr, state="OPEN", reviewers=None):
    result = dict(id=pr, state=state, reviewers=[me] if reviewers is None else reviewers)
    if mode in {"author_team", "author_account"}:
        result["author"] = author(pr)
        if pr == 2:
            result["participants"] = [{"user": me, "approved": True}]
    return result


class Api(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def respond(self, code, body, retry_after=None):
        payload = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        if retry_after is not None:
            self.send_header("Retry-After", str(retry_after))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        self.route()

    def do_POST(self):
        self.route()

    def route(self):
        global truncated_page, cooldown
        credential = self.headers.get("Authorization")
        valid = "Bearer AUTH_SECRET_MUST_NEVER_APPEAR"
        if credential not in {valid, valid + "-renewed"}:
            return self.respond(401, {})
        renewed = credential.endswith("-renewed")
        url = urllib.parse.urlsplit(self.path)
        path, query = url.path, urllib.parse.parse_qs(url.query)
        log.write(json.dumps([self.command, path]) + "\n")
        if mode in {"rate_partial", "rate_after_commit"} and cooldown and time.monotonic() < cooldown:
            log.write(json.dumps(["EARLY", path]) + "\n")
            return self.respond(429, {}, 300)
        if path == "/2.0/user" and self.command == "GET":
            return self.respond(200, other if mode == "identity_change" and renewed else me)
        if mode in {"auth_expiry", "identity_change", "unchanged_token"} and path.endswith("/8/approve") and not renewed:
            (root / "oauth-expired").touch()
            return self.respond(401, {})
        if mode == "concurrent_expiry" and not renewed and re.search(r"/[12]/activity$", path):
            try:
                parallel_reads.wait()
            except threading.BrokenBarrierError:
                return self.respond(500, {})
            (root / "oauth-expired").touch()
            return self.respond(401, {})
        if (root / "oauth-expired").exists() and not renewed:
            return self.respond(401, {})
        if mode == "workspace_pool" and self.command == "GET":
            if path == "/2.0/user/workspaces":
                return self.respond(200, {"values": [
                    {"workspace": {"slug": f"workspace-{i:02d}"}} for i in range(13)
                ]})
            match = re.fullmatch(r"/2.0/workspaces/(workspace-(\d{2}))/pullrequests/(%7B.+%7D)", path)
            if match:
                workspace, index, user = match.groups()
                if urllib.parse.unquote(user) != me["uuid"] or query.get("state") != ["OPEN"]:
                    return self.respond(400, {})
                if index == "00" and not pool_advanced.wait(5):
                    return self.respond(409, {})
                if index == "01" and index not in limits:
                    limits.add(index)
                    return self.respond(429, {}, 1)
                if index == "12":
                    pool_advanced.set()
                return self.respond(200, {"values": [{
                    "id": 1, "title": workspace, "state": "OPEN", "author": me,
                    "destination": {"repository": {"full_name": f"{workspace}/repo"}},
                }]})
        if mode.startswith("discover_review_requests") and self.command == "GET":
            if path == "/2.0/user/workspaces":
                if query.get("fields") != ["next,values.workspace.slug"]:
                    return self.respond(400, {})
                page_two = query.get("page") == ["2"]
                workspace = "other-team" if page_two else "team"
                body = {"values": [{"workspace": {"slug": workspace}}]}
                if not page_two:
                    body["next"] = origin + path + "?page=2&fields=next,values.workspace.slug"
                return self.respond(200, body)
            match = re.fullmatch(r"/2.0/workspaces/(team|other-team)/pullrequests/(%7B.+%7D)", path)
            if match:
                workspace, user = match.groups()
                if urllib.parse.unquote(user) != me["uuid"] or query.get("state") != ["OPEN"]:
                    return self.respond(400, {})
                fields = "next,values.id,values.state,values.title,values.author.uuid,values.destination.repository.full_name"
                if query.get("fields") != [fields]:
                    return self.respond(400, {})
                if mode == "discover_review_requests_empty":
                    return self.respond(200, {"values": []})
                if mode == "discover_review_requests" and "page" not in query:
                    try:
                        parallel_reads.wait()
                    except threading.BrokenBarrierError:
                        return self.respond(500, {})
                def row(repo, number, title, author=me, state="OPEN"):
                    return {"id": number, "title": title, "state": state, "author": author,
                            "destination": {"repository": {"full_name": repo}}}
                if workspace == "other-team":
                    return self.respond(200, {"values": [row("other-team/three", 8, "Checks the other workspace")]})
                first = row("team/one", 8, "Preserves the current title")
                if query.get("page") == ["2"]:
                    return self.respond(200, {"values": [
                        first, row("team/two", 8, "Preserves the second repository"),
                        row("team/one", 9, "Closed", state="MERGED"),
                        row("team/one", 10, "Someone else's", author=other),
                    ]})
                next_page = origin + path + "?state=OPEN&page=2&fields=" + fields
                if mode == "discover_review_requests_bad_page":
                    next_page = origin + "/2.0/user/workspaces?page=2"
                return self.respond(200, {"values": [first], "next": next_page})
        route = re.fullmatch(base + r"team/(one|two|unlisted|bad)/pullrequests(?:/(\d+)(/activity|/approve)?)?", path)
        if not route:
            return self.respond(404, {})
        repo, pr, suffix = route.groups()
        if pr is None and self.command == "GET":
            if query.get("state") != ["OPEN"]:
                return self.respond(400, {})
            if "page" not in query:
                selector = 'reviewers.uuid="' + me["uuid"] + '"'
                if mode == "author_team":
                    selector += ' AND author.account_id IN ("first-member","second-member")'
                if mode == "author_account":
                    selector += ' AND author.account_id="first-member"'
                if query.get("q") != [selector]:
                    return self.respond(400, {})
            if mode in {"author_team", "author_account"} and "page" not in query:
                if "values.author.account_id" not in query.get("fields", [""])[0].split(","):
                    return self.respond(400, {})
            if repo == "unlisted":
                return self.respond(200, {"values": [candidate(10)]})
            if mode == "discovery_overlap" and repo in {"one", "two"} and "page" not in query:
                try:
                    parallel_reads.wait()
                except threading.BrokenBarrierError:
                    return self.respond(500, {})
            if repo == "bad":
                return self.respond(403, {})
            if mode == "rate_long":
                return self.respond(429, {}, 300)
            if mode == "rate_partial" and "list" not in limits:
                limits.add("list")
                cooldown = time.monotonic() + 1
                return self.respond(429, {}, 1)
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
                if mode in {"author_team", "author_account"}:
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
        if mode == "print_review_request" and suffix is None and self.command == "GET":
            if query.get("fields") != ["id,state,title"]:
                return self.respond(400, {})
            if pr == 404:
                return self.respond(404, {})
            if pr == 9:
                return self.respond(200, dict(id=pr, state="MERGED", title="Already merged"))
            if pr == 10:
                return self.respond(200, dict(id=11, state="OPEN", title="Wrong pull request"))
            if pr == 11:
                return self.respond(200, dict(id=pr, state="OPEN", title=" \t\n"))
            title = {
                ("two", 8): "Logs the request and trace IDs",
                ("one", 1): "Reuses the owner’s lease\nwithout an extra Redis round trip",
                ("one", 8): "Handles \u001b control characters",
            }[(repo, pr)]
            return self.respond(200, dict(id=pr, state="OPEN", title=title))
        if suffix == "/activity" and self.command == "GET":
            if mode == "overlap" and pr in {1, 2}:
                try:
                    parallel_reads.wait()
                except threading.BrokenBarrierError:
                    return self.respond(500, {})
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
            if mode in {"rate_partial", "rate_after_commit"} and pr == 8 and "write" not in limits:
                limits.add("write")
                cooldown = time.monotonic() + 1
                if mode == "rate_after_commit":
                    approved.add(pr)
                return self.respond(429, {}, 1)
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
            return self.respond(200, dict(body, author=author(pr, True) if mode in {"author_team", "author_account"} else other,
                                          participants=[participant]))
        return self.respond(405, {})


server_type = http.server.ThreadingHTTPServer if mode in {"overlap", "discovery_overlap", "concurrent_expiry", "discover_review_requests", "workspace_pool"} else http.server.HTTPServer
if mode == "workspace_pool":
    server_type.request_queue_size = 32
with server_type(("127.0.0.1", 0), Api) as server:
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(root / "cert.pem", root / "key.pem")
    server.socket = ctx.wrap_socket(server.socket, server_side=True)
    print(server.server_port, flush=True)
    server.serve_forever()
