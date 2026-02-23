export type Platform = 'github' | 'trello' | 'jira';

export type EscrowStatus = 'active' | 'completed' | 'cancelled';

export type MilestoneStatus =
  | 'pending'
  | 'pending_release'
  | 'released'
  | 'disputed'
  | 'refunded';

export interface TaskEvent {
  platform:    Platform;
  externalId:  string;   // repo full_name, board ID, project key
  taskId:      string;   // issue number, card ID, issue key
  taskTitle:   string;
  taskLabels:  string[];
  taskUrl:     string;
  isDone:      boolean;
  rawPayload:  unknown;
}

export interface Escrow {
  id:                string;
  contract_id:       string;
  token_id:          string;
  client_address:    string;
  developer_address: string;
  total_amount:      bigint;
  status:            EscrowStatus;
  platform:          Platform;
  repo_or_board:     string;
  created_at:        Date;
}

export interface Milestone {
  id:              string;
  escrow_id:       string;
  index:           number;
  title:           string;
  trigger_keyword: string;
  bps:             number;
  status:          MilestoneStatus;
  completed_at:    Date | null;
  task_url:        string | null;
}
