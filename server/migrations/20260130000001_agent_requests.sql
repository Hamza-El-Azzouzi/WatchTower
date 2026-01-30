-- Agent Request Table - stores user requests for agent quota
CREATE TABLE IF NOT EXISTS agent_requests (
    id BIGSERIAL PRIMARY KEY,
    company_name TEXT NOT NULL,
    contact_name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT,
    agents_requested INTEGER NOT NULL,
    use_case TEXT NOT NULL,
    message TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'contacted')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    reviewed_by TEXT,
    reviewed_at TIMESTAMP WITH TIME ZONE,
    notes TEXT
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_agent_requests_status ON agent_requests(status);
CREATE INDEX IF NOT EXISTS idx_agent_requests_email ON agent_requests(email);
CREATE INDEX IF NOT EXISTS idx_agent_requests_created_at ON agent_requests(created_at DESC);
