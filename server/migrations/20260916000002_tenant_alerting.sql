ALTER TABLE alert_rules ADD COLUMN owner_api_key_id BIGINT REFERENCES api_keys(id);
ALTER TABLE notification_channels ADD COLUMN owner_api_key_id BIGINT REFERENCES api_keys(id);
ALTER TABLE alert_silences ADD COLUMN owner_api_key_id BIGINT REFERENCES api_keys(id);
ALTER TABLE notification_deliveries ADD COLUMN owner_api_key_id BIGINT REFERENCES api_keys(id);
ALTER TABLE alert_rules DROP CONSTRAINT IF EXISTS alert_rules_name_key;
CREATE UNIQUE INDEX alert_rules_tenant_name_idx ON alert_rules(owner_api_key_id,name);
ALTER TABLE notification_channels DROP CONSTRAINT IF EXISTS notification_channels_name_key;
CREATE UNIQUE INDEX notification_channels_tenant_name_idx ON notification_channels(owner_api_key_id,name);
CREATE INDEX alert_rules_tenant_idx ON alert_rules(owner_api_key_id);
CREATE INDEX notification_channels_tenant_idx ON notification_channels(owner_api_key_id);
CREATE INDEX alert_silences_tenant_idx ON alert_silences(owner_api_key_id);
CREATE INDEX notification_deliveries_tenant_idx ON notification_deliveries(owner_api_key_id,created_at DESC);
-- Preserve existing rules when their exact agent proves tenant ownership.
UPDATE alert_rules r SET owner_api_key_id=a.api_key_id FROM agents a WHERE r.agent_filter=a.id;
-- Unscoped legacy rules remain stored but cannot run across enterprise boundaries.
UPDATE alert_rules SET enabled=false WHERE owner_api_key_id IS NULL;
WITH owners AS (
 SELECT channel_id, min(owner_api_key_id) AS owner FROM alert_rules,
 LATERAL jsonb_array_elements_text(channels) AS channel_id
 WHERE owner_api_key_id IS NOT NULL GROUP BY channel_id HAVING count(DISTINCT owner_api_key_id)=1
) UPDATE notification_channels c SET owner_api_key_id=o.owner FROM owners o WHERE c.id=o.channel_id;
UPDATE notification_channels SET enabled=false WHERE owner_api_key_id IS NULL;
UPDATE alert_silences s SET owner_api_key_id=r.owner_api_key_id FROM alert_rules r WHERE s.rule_id=r.rule_id;
UPDATE alert_silences s SET owner_api_key_id=a.api_key_id FROM agents a WHERE s.owner_api_key_id IS NULL AND s.agent_id=a.id;
UPDATE notification_deliveries d SET owner_api_key_id=c.owner_api_key_id FROM notification_channels c WHERE d.channel_id=c.id;
