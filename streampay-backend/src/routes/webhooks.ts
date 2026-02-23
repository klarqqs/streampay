import { Router, Request, Response } from 'express';
import { handleGithubWebhook } from '../adapters/github';

export const webhookRouter = Router();

webhookRouter.post('/github', async (req: Request, res: Response) => {
  const event  = req.headers['x-github-event'] as string;
  const action = req.body?.action;
  console.log(`📨 GitHub event: ${event} | action: ${action}`);
  await handleGithubWebhook(req, res);
});

webhookRouter.post('/:platform', async (req: Request, res: Response) => {
  const { platform } = req.params;
  console.log(`📨 Webhook received from: ${platform} (adapter coming soon)`);
  res.json({ received: true, platform });
});
