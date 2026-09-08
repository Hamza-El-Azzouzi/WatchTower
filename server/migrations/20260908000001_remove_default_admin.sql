-- Remove the publicly documented bootstrap account if it was never used.
-- Administrators are now bootstrapped explicitly from environment variables.
DELETE FROM admin_users
WHERE username = 'admin'
  AND password_hash = '$2b$12$mFYO.MsW/zucU1fIDg/vD.Eqyfso4Phf2YzGGFJZcX7yCyDb/cOIu'
  AND last_login_at IS NULL;
