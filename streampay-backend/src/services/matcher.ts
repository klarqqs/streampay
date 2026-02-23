import { db } from '../db/client';
import { TaskEvent } from '../types';
import { markComplete } from './stellar';

export async function processTaskEvent(event: TaskEvent): Promise<void> {
  if (!event.isDone) return;

  console.log(`🔍 Processing task: "${event.taskTitle}" from ${event.platform}`);

  // Find platform connection
  const conn = await db.query(
    `SELECT pc.*, e.contract_id, e.status as escrow_status
     FROM platform_connections pc
     JOIN escrows e ON e.id = pc.escrow_id
     WHERE pc.platform = $1 AND pc.external_id = $2`,
    [event.platform, event.externalId]
  );

  if (!conn.rows[0]) {
    console.log(`⚠️  No escrow found for ${event.platform}:${event.externalId}`);
    return;
  }

  const { escrow_id, contract_id, escrow_status } = conn.rows[0];

  if (escrow_status !== 'active') {
    console.log(`⚠️  Escrow ${escrow_id} is not active (${escrow_status})`);
    return;
  }

  // Find matching milestone
  const milestones = await db.query(
    `SELECT * FROM milestones
     WHERE escrow_id = $1 AND status = 'pending'
     ORDER BY index`,
    [escrow_id]
  );

  const milestone = milestones.rows.find(m =>
    event.taskTitle.toLowerCase().includes(m.trigger_keyword.toLowerCase()) ||
    event.taskLabels.some(l => l.toLowerCase().includes(m.trigger_keyword.toLowerCase()))
  );

  if (!milestone) {
    console.log(`⚠️  No matching milestone for "${event.taskTitle}"`);
    return;
  }

  console.log(`🎯 Matched milestone: "${milestone.title}" (index ${milestone.index})`);

  // Submit to Stellar
  const txHash = await markComplete(contract_id, milestone.index, event.taskUrl);

  // Update DB
  await db.query(
    `UPDATE milestones
     SET status = 'pending_release', completed_at = NOW(), task_url = $1
     WHERE id = $2`,
    [event.taskUrl, milestone.id]
  );

  console.log(`✅ Milestone ${milestone.index} marked complete. TX: ${txHash}`);
}
