import { createHmac } from 'crypto';
import { Request, Response } from 'express';
import { TaskEvent } from '../types';
import { processTaskEvent } from '../services/matcher';

function verifySignature(req: Request): boolean {
  const secret    = process.env.GITHUB_WEBHOOK_SECRET!;
  const signature = req.headers['x-hub-signature-256'] as string;
  if (!signature) return false;

  const hmac     = createHmac('sha256', secret);
  const digest   = 'sha256=' + hmac.update(JSON.stringify(req.body)).digest('hex');
  return signature === digest;
}

export async function handleGithubWebhook(req: Request, res: Response): Promise<void> {
  // Verify signature
  if (!verifySignature(req)) {
    res.status(401).json({ error: 'Invalid signature' });
    return;
  }

  const event  = req.headers['x-github-event'] as string;
  const payload = req.body;

  console.log(`📨 GitHub event: ${event}`);

  // Only handle project card events
  if (event !== 'projects_v2_item') {
    res.json({ skipped: true, reason: 'not a project item event' });
    return;
  }

  // Only handle items moved to Done
  const action     = payload.action;
  const fieldValue = payload.changes?.field_value?.to;

  if (action !== 'edited' || fieldValue?.name !== 'Done') {
    res.json({ skipped: true, reason: 'not a Done transition' });
    return;
  }

  const item       = payload.projects_v2_item;
  const repoName   = payload.repository?.full_name || payload.organization?.login;
  const taskTitle  = item.content?.title || '';
  const taskUrl    = item.content?.url   || '';
  const taskLabels = item.content?.labels?.map((l: any) => l.name) || [];

  const taskEvent: TaskEvent = {
    platform:   'github',
    externalId: repoName,
    taskId:     String(item.id),
    taskTitle,
    taskLabels,
    taskUrl,
    isDone:     true,
    rawPayload: payload,
  };

  // Process async — respond immediately to GitHub
  res.json({ received: true });
  await processTaskEvent(taskEvent);
}
