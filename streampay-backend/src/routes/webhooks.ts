import { Router, Request, Response } from 'express';
import { handleGithubWebhook } from '../adapters/github';

export const webhookRouter = Router();

webhookRouter.post('/github', handleGithubWebhook);

webhookRouter.post('/:platform', async (req: Request, res: Response) => {
  const { platform } = req.params;
  console.log(`📨 Webhook received from: ${platform} (adapter coming soon)`);
  res.json({ received: true, platform });
});
