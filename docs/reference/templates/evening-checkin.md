# Evening Check-in Template

Schedule as a cron routine (e.g., 4:30 PM or 6:00 PM).

```markdown
## Evening Check-in — {{date}}

### What got done today?
-

### What didn't get done? Why?
-

### Plan for tomorrow (top 3)
1.
2.
3.

### Overnight work queue
- [Tasks the agent should tackle while you rest]

### Blockers or decisions pending?
-

### Energy/mood (1-10)
-
```

## Cron Configuration

```json
{
  "name": "evening-checkin",
  "cron": "30 16 * * *",
  "session": "isolated",
  "message": "Add the evening check-in template to today's daily log. Pre-fill what you know from today's activity. Alert me that it's ready for review.",
  "announce": true,
  "channel": "telegram"
}
```
