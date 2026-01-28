# Admin Authentication System

## Overview
The DevOps Monitoring System now has separate authentication for administrators and regular monitoring users. Admins can manage API keys and view all system data without being involved in monitoring themselves.

## Authentication Types

### 1. Admin Authentication
**Purpose**: Dashboard administration and API key management  
**Access**: Admin dashboard, API key CRUD operations, system overview  
**Credentials**: Username and password  
**Default Login**: `admin` / `admin123`

### 2. Regular User Authentication  
**Purpose**: Monitoring agents and viewing metrics  
**Access**: Server/database metrics, alerts, performance dashboards  
**Credentials**: API key (e.g., `msk_xxx...`)

## Admin Features

### Admin Dashboard (`/admin`)
- Overview of all API keys and their status
- Total agent count (servers + databases)
- Active alerts monitoring
- Usage statistics per API key
- Agent distribution visualization

### API Key Management (`/admin/api-keys`)
- Create new API keys with custom settings
- Set expiration dates and agent limits
- View which agents use each key
- Revoke keys instantly
- Track last usage timestamps

## Login Pages

### Admin Login (`/admin-login`)
```
URL: http://localhost:3000/admin-login
Credentials:
  - Username: admin
  - Password: admin123
```

### Regular User Login (`/login`)
```
URL: http://localhost:3000/login
Credentials:
  - API Key: msk_xxx... (get from admin dashboard)
```

## Database Schema

### Admin Users Table
```sql
CREATE TABLE admin_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    email TEXT,
    full_name TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_login_at TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT 1
);
```

## API Endpoints

### Admin Authentication
```
POST /api/v1/admin/login
Body: { "username": "admin", "password": "admin123" }
Response: { "username": "admin", "full_name": "System Administrator", "token": "admin_xxx..." }

GET /api/v1/admin/validate
Headers: X-Admin-Token: admin_xxx...
Response: { "valid": true }
```

### API Key Management (Admin Only)
```
POST /api/v1/auth/keys
GET /api/v1/auth/keys
DELETE /api/v1/auth/keys/:key_id
```

## Security Notes

1. **Change Default Password**: Immediately change the default admin password after first login
2. **Admin Token Storage**: Stored in browser localStorage with key `admin_token`
3. **Password Hashing**: All passwords are hashed using bcrypt before storage
4. **Session Management**: Admin tokens are generated per session
5. **Route Protection**: Admin routes automatically redirect non-admin users

## Usage Examples

### 1. Admin Login Flow
```bash
# Admin logs in
curl -X POST http://localhost:8080/api/v1/admin/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'

# Response includes token
{
  "username": "admin",
  "full_name": "System Administrator",
  "token": "admin_abc123xyz..."
}
```

### 2. Create API Key as Admin
```bash
# Create new API key for monitoring
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -H "X-Admin-Token: admin_abc123xyz..." \
  -d '{
    "name": "production-servers",
    "description": "Keys for production server agents",
    "expires_in_days": 90,
    "max_agents": 10
  }'
```

### 3. Regular User Accesses Dashboard
```bash
# User logs in with API key
# Navigate to http://localhost:3000/login
# Enter API key: msk_xxx...
# Can only see servers/databases associated with their key
```

## Migration Path

If you're upgrading from the previous version:

1. **Run the migration**:
   ```bash
   cd server
   sqlite3 monitoring.db < migrations/004_admin_users.sql
   ```

2. **Verify admin user**:
   ```bash
   sqlite3 monitoring.db "SELECT * FROM admin_users;"
   ```

3. **Login to admin dashboard**:
   - Navigate to `http://localhost:3000/admin-login`
   - Use credentials: `admin` / `admin123`
   - Change password immediately

## Key Differences

| Feature | Admin | Regular User |
|---------|-------|--------------|
| Login Method | Username/Password | API Key |
| Access to /admin | ✅ Yes | ❌ No |
| Create API Keys | ✅ Yes | ❌ No |
| View All Agents | ✅ Yes | ❌ No (only their agents) |
| Monitoring Agents | ❌ No agents associated | ✅ Agents tracked |
| Dashboard Access | ✅ Full access | ✅ Filtered by API key |

## Benefits

1. **Separation of Concerns**: Admins manage the system, users monitor their infrastructure
2. **Multi-Tenant Support**: Each API key sees only their data
3. **Centralized Management**: Single admin account manages all monitoring users
4. **Security**: Different authentication mechanisms for different roles
5. **Audit Trail**: Track which admin created which API keys

## Troubleshooting

### Cannot login as admin
- Verify migration ran successfully: `sqlite3 monitoring.db "SELECT * FROM admin_users;"`
- Check server logs for authentication errors
- Ensure server is running: `cargo run` in server directory

### Admin token not working
- Clear browser localStorage and login again
- Check X-Admin-Token header is being sent with requests
- Verify token format starts with "admin_"

### Regular user sees admin page
- Check localStorage: `user_type` should be empty or "user", not "admin"
- Clear localStorage and login again with API key
