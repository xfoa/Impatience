#!/usr/bin/env python3
"""
Simulate the exact TimeSync algorithm to understand what correction_ms
converges to when start times differ and latency is present.
"""

import random

def simulate(start_c: int, start_s: int, lat_cs_us: int, lat_sc_us: int, seed=42):
    rng = random.Random(seed)

    def local_c(t):
        return t - start_c

    def local_s(t):
        return t - start_s

    def ts24(t, start):
        return ((t - start) >> 3) & 0xFFFFFF

    # Simulate many probe exchanges
    min_recv_c = float('inf')
    min_recv_s = float('inf')
    samples_c = []
    samples_s = []

    for i in range(200):
        # Client sends ping at time t
        t = max(start_c, start_s) + 50000 + i * 10000 + rng.randint(0, 500)

        # Server receives ping
        t_recv_s = t + lat_cs_us
        send_ts24_c = ts24(t, start_c)
        recv_ts24_s = ts24(t_recv_s, start_s)

        # delta24 at server: recv_ts24_s - send_ts24_c (with wrap)
        delta_s = (recv_ts24_s - send_ts24_c) & 0xFFFFFF
        samples_s.append(delta_s)
        min_recv_s = min(min_recv_s, delta_s)

        # Server sends pong (immediate echo for simplicity)
        t_send_s = t_recv_s
        t_recv_c = t_send_s + lat_sc_us
        send_ts24_s = ts24(t_send_s, start_s)
        recv_ts24_c = ts24(t_recv_c, start_c)

        delta_c = (recv_ts24_c - send_ts24_s) & 0xFFFFFF
        samples_c.append(delta_c)
        min_recv_c = min(min_recv_c, delta_c)

    # Now simulate sync exchange
    # Client sends min_recv_c to server
    # Server sends min_recv_s to client

    # At server:
    min_send_s = min_recv_c  # the value the client told the server
    clock_delta_s = ((min_send_s - min_recv_s) & 0xFFFFFF) >> 1
    # Sign handling for 23-bit
    if clock_delta_s >= 0x400000:
        clock_delta_s -= 0x800000
    clock_delta_s_usec = clock_delta_s << 3
    correction_s_ms = clock_delta_s_usec / 1000

    # At client:
    min_send_c = min_recv_s
    clock_delta_c = ((min_send_c - min_recv_c) & 0xFFFFFF) >> 1
    if clock_delta_c >= 0x400000:
        clock_delta_c -= 0x800000
    clock_delta_c_usec = clock_delta_c << 3
    correction_c_ms = clock_delta_c_usec / 1000

    # Compute actual quantities
    start_delta_ms = (start_s - start_c) / 1000  # server start minus client start
    lat_ms = (lat_cs_us + lat_sc_us) / 2000  # average latency in ms

    print(f"start_c={start_c}, start_s={start_s}, lat_cs={lat_cs_us}us, lat_sc={lat_sc_us}us")
    print(f"  start_delta_ms (server - client) = {start_delta_ms}")
    print(f"  avg latency ms = {lat_ms}")
    print(f"  min_recv_c = {min_recv_c} (ts24 units)")
    print(f"  min_recv_s = {min_recv_s} (ts24 units)")
    print(f"  correction_c_ms = {correction_c_ms}")
    print(f"  correction_s_ms = {correction_s_ms}")

    # What should remote_ms be?
    # At time t, server local_ms = (t - start_s)/1000
    # At time t, client local_ms = (t - start_c)/1000
    # Server wants remote_ms = client_local_ms = server_local_ms + start_delta_ms
    # Client wants remote_ms = server_local_ms = client_local_ms - start_delta_ms

    # If server does: remote_ms = local_ms + correction_s_ms + start_delta_ms
    # = server_local_ms + correction_s_ms + start_delta_ms
    # Should equal client_local_ms = server_local_ms + start_delta_ms
    # So correction_s_ms should be 0 for this to work

    # If server does: remote_ms = local_ms + correction_s_ms
    # = server_local_ms + correction_s_ms
    # Should equal client_local_ms = server_local_ms + start_delta_ms
    # So correction_s_ms should equal start_delta_ms

    # Let's check what correction_s_ms actually equals
    print(f"  Does correction_s_ms == start_delta_ms? {correction_s_ms} == {start_delta_ms} -> {abs(correction_s_ms - start_delta_ms) < 0.1}")
    print(f"  Does correction_c_ms == -start_delta_ms? {correction_c_ms} == {-start_delta_ms} -> {abs(correction_c_ms + start_delta_ms) < 0.1}")

    # Test: at a random time t
    t = max(start_c, start_s) + 1000000
    server_local_ms = (t - start_s) / 1000
    client_local_ms = (t - start_c) / 1000

    # Method A: remote_ms = local_ms + correction_ms
    remote_from_server_A = server_local_ms + correction_s_ms
    remote_from_client_A = client_local_ms + correction_c_ms
    print(f"\n  At t={t}:")
    print(f"    server_local_ms={server_local_ms}, client_local_ms={client_local_ms}")
    print(f"    Method A (local + correction):")
    print(f"      server->client remote_ms = {remote_from_server_A} (target={client_local_ms}, error={remote_from_server_A - client_local_ms})")
    print(f"      client->server remote_ms = {remote_from_client_A} (target={server_local_ms}, error={remote_from_client_A - server_local_ms})")

    # Method B: remote_ms = local_ms + correction_ms + start_delta_ms
    remote_from_server_B = server_local_ms + correction_s_ms + start_delta_ms
    remote_from_client_B = client_local_ms + correction_c_ms - start_delta_ms  # client uses -start_delta_ms
    print(f"    Method B (local + correction + start_delta):")
    print(f"      server->client remote_ms = {remote_from_server_B} (target={client_local_ms}, error={remote_from_server_B - client_local_ms})")
    print(f"      client->server remote_ms = {remote_from_client_B} (target={server_local_ms}, error={remote_from_client_B - server_local_ms})")

    # Method C: remote_ms = local_ms + start_delta_ms (ignore correction)
    remote_from_server_C = server_local_ms + start_delta_ms
    remote_from_client_C = client_local_ms - start_delta_ms
    print(f"    Method C (local + start_delta only):")
    print(f"      server->client remote_ms = {remote_from_server_C} (target={client_local_ms}, error={remote_from_server_C - client_local_ms})")
    print(f"      client->server remote_ms = {remote_from_client_C} (target={server_local_ms}, error={remote_from_client_C - server_local_ms})")


if __name__ == "__main__":
    print("Case 1: Symmetric 10ms latency, no start offset")
    simulate(1000000, 1000000, 10000, 10000)

    print("\nCase 2: Symmetric 10ms latency, server starts 10ms after client")
    simulate(1000000, 1010000, 10000, 10000)

    print("\nCase 3: Asymmetric latency (5ms/15ms), server starts 15ms after client")
    simulate(1000000, 1015000, 5000, 15000)

    print("\nCase 4: Asymmetric latency (15ms/5ms), server starts 5ms after client")
    simulate(1000000, 1005000, 15000, 5000)
