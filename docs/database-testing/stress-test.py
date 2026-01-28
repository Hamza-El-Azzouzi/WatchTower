#!/usr/bin/env python3
"""
Interactive PostgreSQL stress tester with real-time monitoring
"""

import psycopg2
import time
import threading
import random
from datetime import datetime

# Database connection parameters
DB_PARAMS = {
    'host': 'localhost',
    'port': 5432,
    'database': 'testdb',
    'user': 'postgres',
    'password': 'postgres'
}

def create_connection():
    return psycopg2.connect(**DB_PARAMS)

def setup_test_table():
    """Create test table if not exists"""
    conn = create_connection()
    cur = conn.cursor()
    cur.execute("""
        CREATE TABLE IF NOT EXISTS load_test (
            id SERIAL PRIMARY KEY,
            data TEXT,
            value INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_load_value ON load_test(value);
    """)
    conn.commit()
    cur.close()
    conn.close()
    print("✅ Test table created")

def insert_worker(worker_id, num_inserts):
    """Worker thread that performs inserts"""
    conn = create_connection()
    cur = conn.cursor()
    
    for i in range(num_inserts):
        try:
            cur.execute(
                "INSERT INTO load_test (data, value) VALUES (%s, %s)",
                (f"worker_{worker_id}_data_{i}", random.randint(1, 1000))
            )
            conn.commit()
        except Exception as e:
            print(f"❌ Worker {worker_id} error: {e}")
            conn.rollback()
    
    cur.close()
    conn.close()
    print(f"✅ Worker {worker_id} completed {num_inserts} inserts")

def slow_query_worker(worker_id, duration):
    """Worker that runs intentionally slow queries"""
    conn = create_connection()
    cur = conn.cursor()
    
    try:
        # Slow query with sleep
        cur.execute(f"SELECT pg_sleep({duration}), COUNT(*) FROM load_test WHERE data LIKE '%data%'")
        cur.fetchall()
        print(f"🐌 Slow query {worker_id} completed ({duration}s)")
    except Exception as e:
        print(f"❌ Slow query {worker_id} error: {e}")
    finally:
        cur.close()
        conn.close()

def connection_holder(conn_id, hold_time):
    """Hold a connection open"""
    conn = create_connection()
    cur = conn.cursor()
    
    try:
        cur.execute(f"SELECT 'Connection {conn_id}' as name, pg_sleep({hold_time})")
        print(f"🔗 Connection {conn_id} held for {hold_time}s")
    except Exception as e:
        print(f"❌ Connection {conn_id} error: {e}")
    finally:
        cur.close()
        conn.close()

def main():
    print("🔥 PostgreSQL Advanced Stress Test")
    print("=" * 50)
    print()
    
    # Setup
    print("Setting up test environment...")
    setup_test_table()
    print()
    
    print("Choose stress test scenario:")
    print("1. Light load (5 connections, few queries)")
    print("2. Medium load (15 connections, moderate queries)")
    print("3. Heavy load (30 connections, many slow queries)")
    print("4. Custom load")
    print()
    
    choice = input("Enter choice (1-4) [default: 2]: ").strip() or "2"
    
    scenarios = {
        "1": {"workers": 5, "inserts": 100, "slow_queries": 2, "connections": 5, "hold_time": 3},
        "2": {"workers": 10, "inserts": 200, "slow_queries": 5, "connections": 15, "hold_time": 5},
        "3": {"workers": 20, "inserts": 500, "slow_queries": 10, "connections": 30, "hold_time": 8},
    }
    
    if choice in scenarios:
        config = scenarios[choice]
    else:
        # Custom
        config = {
            "workers": int(input("Number of insert workers (default 10): ") or 10),
            "inserts": int(input("Inserts per worker (default 200): ") or 200),
            "slow_queries": int(input("Number of slow queries (default 5): ") or 5),
            "connections": int(input("Number of idle connections (default 15): ") or 15),
            "hold_time": int(input("Hold time in seconds (default 5): ") or 5),
        }
    
    print()
    print(f"🚀 Starting stress test with:")
    print(f"   - {config['workers']} insert workers x {config['inserts']} inserts")
    print(f"   - {config['slow_queries']} slow queries")
    print(f"   - {config['connections']} held connections for {config['hold_time']}s")
    print()
    print("📊 Open dashboard: http://localhost:3000/databases")
    print("⏱️  Monitor metrics in real-time!")
    print()
    
    input("Press Enter to start...")
    
    threads = []
    
    # Start insert workers
    print("\n📝 Starting insert workers...")
    for i in range(config['workers']):
        t = threading.Thread(target=insert_worker, args=(i, config['inserts']))
        t.start()
        threads.append(t)
    
    time.sleep(1)
    
    # Start slow query workers
    print(f"\n🐌 Starting {config['slow_queries']} slow queries...")
    for i in range(config['slow_queries']):
        duration = random.randint(2, 5)
        t = threading.Thread(target=slow_query_worker, args=(i, duration))
        t.start()
        threads.append(t)
    
    time.sleep(1)
    
    # Start connection holders
    print(f"\n🔗 Opening {config['connections']} connections...")
    for i in range(config['connections']):
        t = threading.Thread(target=connection_holder, args=(i, config['hold_time']))
        t.start()
        threads.append(t)
    
    # Wait for all threads
    print("\n⏳ Test running... (this will take a few seconds)")
    for t in threads:
        t.join()
    
    print("\n✅ Stress test completed!")
    print("\n📊 Check the dashboard to see the metrics spike!")
    
    # Show final stats
    try:
        conn = create_connection()
        cur = conn.cursor()
        cur.execute("""
            SELECT 
                count(*) as total_rows,
                pg_size_pretty(pg_table_size('load_test')) as table_size,
                pg_size_pretty(pg_database_size(current_database())) as db_size
            FROM load_test
        """)
        row = cur.fetchone()
        print(f"\n📈 Final Stats:")
        print(f"   - Total rows: {row[0]}")
        print(f"   - Table size: {row[1]}")
        print(f"   - Database size: {row[2]}")
        cur.close()
        conn.close()
    except Exception as e:
        print(f"❌ Error fetching stats: {e}")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\n⚠️  Test interrupted by user")
    except Exception as e:
        print(f"\n❌ Error: {e}")
