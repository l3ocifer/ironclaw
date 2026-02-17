# Boot Checklist

Run these checks on every gateway restart, before doing anything else.

## Environment Health
- [ ] Verify disk space > 10% free
- [ ] Check database connectivity
- [ ] Verify API key validity (test a minimal completion)

## Workspace Integrity
- [ ] Read `MEMORY.md` — confirm it's not corrupted or empty
- [ ] Read today's `daily/YYYY-MM-DD.md` — resume context
- [ ] Check `active-tasks.md` — resume any in-progress work

## Recovery
- [ ] If active-tasks.md has in-progress items, summarize what was interrupted
- [ ] If yesterday's evening check-in is missing, note it for the morning report

## Security
- [ ] Verify identity files (SOUL.md, AGENTS.md) haven't been tampered with
- [ ] Check for unusual files in workspace root

Report findings only if action is needed. Silent boot is a healthy boot.
