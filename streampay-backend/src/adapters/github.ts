import { createHmac } from 'crypto';
import { Request, Response } from 'express';
import { TaskEvent } from '../types';
import { processTaskEvent } from '../services/matcher';

function verifySignature(req: Request): boolean {
  const secret    = process.env.GITHUB_WEBHOOK_SECRET!;
  const signature = req.headers['x-hub-signature-256'] as string;
  if (!signature) return false;
  const hmac    = createHmac('sha256', secret);
  const digest  = 'sha256=' + hmac.update(JSON.stringify(req.body)).digest('hex');
  return signature === digest;
}

export async function handleGithubWebhook(req: Request, res: Response): Promise<void> {
  const event   = req.headers['x-github-event'] as string;
  const payload = req.body;

  // Handle issue closed = Done
  if (event === 'issues' && payload.action === 'closed') {
    const issue = payload.issue;
    const repoName = payload.repository.full_name;

    const taskEvent: TaskEvent = {
      platform:   'github',
      externalId: repoName,
      taskId:     String(issue.number),
      taskTitle:  issue.title,
      taskLabels: issue.labels?.map((l: any) => l.name) || [],
      taskUrl:    issue.html_url,
      isDone:     true,
      rawPayload: payload,
    };

    res.json({ received: true });
    await processTaskEvent(taskEvent);
    return;
  }

  res.json({ received: true, event, skipped: true });
}
