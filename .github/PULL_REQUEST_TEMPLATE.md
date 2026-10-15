## Septuvo change control

**Task ID (base M?-??, dotted subtask M?-??.?, or hygiene H-???):**
**Canonical work order/handoff ([docs/WORK_ORDER_TEMPLATE.md](../docs/WORK_ORDER_TEMPLATE.md)):**
**Category (D-031):** A routine (H-???) · B milestone work (link the owner's authorization) · C gate/security decision
**Owner authorization link (category B only):**
**Predecessor complete (merged PR link):**
**Required gate and owner approval evidence:**  
**Gate satisfied (if applicable):**  
**Starting branch and commit:**  
**Changed files and scoped objective:**  
**Out-of-scope files kept unchanged:**  

### Risk and security impact
- What untrusted inputs or OS privileges are affected?
- Any impact to identity, KDF, nonce, handshake, routing/kill switch, authentication, enrollment or traffic privacy?
- Is a DECISIONS.md entry and external re-review required?

### Tests, rollback and handoff
Use **all** fields in [docs/WORK_ORDER_TEMPLATE.md](../docs/WORK_ORDER_TEMPLATE.md); report PASS/FAIL/NOT RUN with command evidence and whether rollback was tested. Name the task in every commit subject, for example `feat(N1-03): ...`. The owner's merge accepts this pull request; no acceptance comment is needed (D-031). Gates still need the owner's exact `GATE <Gn> PASSED/FAILED` comment. STOP.

**Do not check a box or mark PASS unless the command/result is real. Never claim independent audit without an audit report.**
