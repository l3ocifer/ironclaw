# Morning Report Template

Schedule as a cron routine at your preferred wake time (e.g., 7:00 AM).

```markdown
## Morning Report — {{date}}

### Overnight Summary
- [What the agent completed while you slept]
- [What got stuck and why]

### Decisions Needed
- [Decision 1 — context and options]
- [Decision 2 — context and options]

### Today's Priorities
1. [Carried from yesterday's evening check-in]
2. [New based on overnight findings]
3. [Standing commitment or deadline]

### Reminders
- [Follow-ups, deadlines, events]

### Weather & Calendar
- [Brief weather forecast]
- [Today's calendar highlights]
```

## Cron Configuration

```json
{
  "name": "morning-report",
  "cron": "0 7 * * *",
  "session": "isolated",
  "message": "Generate today's morning report. Check overnight work, review calendar, check weather, write to today's daily log.",
  "announce": true,
  "channel": "telegram"
}
```
