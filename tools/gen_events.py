"""Generate the recorded telemetry events in data/events.jsonl (deterministic)."""

import json
import random
import uuid
from pathlib import Path

SEED = 42
N_VINS = 50
DAYS = 3
START_MS = 1_793_491_200_000  # 2026-11-01T00:00:00Z
DAY_MS = 86_400_000
MIN_MS = 60_000

DUPLICATE_RATE = 0.03  # car resends an event after a dropped ack
LATE_RATE = 0.02  # car was offline and uploads buffered events later

VIN_CHARS = "ABCDEFGHJKLMNPRSTUVWXYZ0123456789"
DTCS = [
    ("P0A80", "warn", "BMS"),
    ("P0AA6", "critical", "BMS"),
    ("U0100", "warn", "VCU"),
    ("B1342", "info", "BCM"),
    ("C0035", "warn", "ABS"),
    ("P1E00", "critical", "INV"),
    ("U0293", "info", "GW"),
]
HOMES = [(37.53, -122.04), (37.77, -122.42), (33.45, -112.07), (37.34, -121.89)]

rng = random.Random(SEED)


def make_id():
    return str(uuid.UUID(int=rng.getrandbits(128)))


def make_vin():
    return "50EA1" + "".join(rng.choice(VIN_CHARS) for _ in range(12))


events = []


def emit(vin, fw, typ, ts, **payload):
    events.append(
        {
            "event_id": make_id(),
            "vin": vin,
            "type": typ,
            "event_ts": int(ts),
            "fw_version": fw,
            "payload": payload,
        }
    )


def simulate_vehicle(vin):
    fw = rng.choice(["2.4.1", "2.5.0", "2.5.2"])
    capacity = rng.choice([92.0, 112.0])
    soh = round(rng.uniform(91.0, 99.5), 2)
    soc = rng.uniform(8, 90)
    odo = rng.uniform(2_000, 60_000)
    home = rng.choice(HOMES)
    base_temp = rng.uniform(18, 30)

    for day in range(DAYS):
        day_start = START_MS + day * DAY_MS
        # battery status every 30 minutes
        for slot in range(48):
            ts = day_start + slot * 30 * MIN_MS + rng.randint(0, 59_000)
            vmin = round(rng.uniform(3.55, 3.75), 3)
            imbalance = rng.uniform(0.01, 0.05) if rng.random() > 0.03 else rng.uniform(0.09, 0.15)
            temp = base_temp + rng.uniform(-4, 8) + (18 if rng.random() < 0.01 else 0)
            emit(vin, fw, "battery_status", ts, soc=round(soc, 2), cell_temp_c=round(temp, 2),
                 cell_voltage_min=vmin, cell_voltage_max=round(vmin + imbalance, 3), soh_pct=soh)
            soh = round(soh - rng.uniform(0, 0.002), 3)

        # trips
        t = day_start + rng.randint(6 * 60, 9 * 60) * MIN_MS
        for _ in range(rng.randint(1, 3)):
            trip_id = make_id()
            minutes = rng.randint(10, 75)
            lat, lon = home[0] + rng.uniform(-0.05, 0.05), home[1] + rng.uniform(-0.05, 0.05)
            emit(vin, fw, "trip_start", t, trip_id=trip_id, odometer_km=round(odo, 1), lat=round(lat, 5), lon=round(lon, 5))
            distance = 0.0
            for m in range(5, minutes, 5):
                speed = max(0.0, rng.gauss(70, 30)) if rng.random() > 0.03 else rng.uniform(121, 160)
                distance += speed * 5 / 60
                lat += rng.uniform(-0.01, 0.01)
                lon += rng.uniform(-0.01, 0.01)
                ts = t + m * MIN_MS
                emit(vin, fw, "location", ts, lat=round(lat, 5), lon=round(lon, 5),
                     speed_kph=round(speed, 1), heading=rng.randint(0, 359))
                if m % 15 == 0:
                    low = rng.random() < 0.04
                    emit(vin, fw, "tire_pressure", ts + 1_000,
                         **{k: round(rng.uniform(165, 195) if low and k == "rl" else rng.uniform(230, 260), 1)
                            for k in ("fl", "fr", "rl", "rr")})
                    signal = rng.uniform(-120, -70)
                    emit(vin, fw, "connectivity", ts + 2_000, signal_dbm=round(signal, 1),
                         network="5G" if rng.random() < 0.55 else "LTE")
                if m % 10 == 0:
                    outside = base_temp + rng.uniform(-6, 10)
                    emit(vin, fw, "climate", ts + 3_000, cabin_temp_c=round(rng.uniform(19, 24), 2),
                         outside_temp_c=round(outside, 2), hvac_power_kw=round(abs(outside - 21) * 0.15 + rng.uniform(0.2, 0.8), 3))
                if rng.random() < 0.04:
                    emit(vin, fw, "adas_event", ts + 4_000, kind=rng.choice(["FCW", "AEB", "LKA", "BSW"]), speed_kph=round(speed, 1))
            odo += distance
            soc = max(5.0, soc - distance * 0.18 * 100 / capacity)
            emit(vin, fw, "trip_end", t + minutes * MIN_MS, trip_id=trip_id, odometer_km=round(odo, 1), lat=round(lat, 5), lon=round(lon, 5))
            t += (minutes + rng.randint(60, 240)) * MIN_MS

        # diagnostics
        for _ in range(rng.choice([0, 1, 1, 2, 3])):
            code, severity, ecu = rng.choice(DTCS)
            emit(vin, fw, "diagnostic", day_start + rng.randint(0, DAY_MS - 1), code=code, severity=severity, ecu=ecu)

        # charging, in the evening
        if soc < 70 or rng.random() < 0.3:
            session_id = make_id()
            dc = rng.random() < 0.25
            power = rng.uniform(120, 220) if dc else rng.uniform(7, 11)
            target = rng.uniform(80, 95) if not dc else rng.uniform(70, 85)
            energy_needed = max(1.0, (target - soc) / 100 * capacity)
            minutes = max(10, int(energy_needed / power * 60))
            t0 = day_start + rng.randint(18 * 60, 21 * 60) * MIN_MS
            emit(vin, fw, "charging_start", t0, session_id=session_id, soc=round(soc, 2),
                 charger_type="DC" if dc else "AC", pack_capacity_kwh=capacity)
            step = 5 if minutes > 30 else 2
            slots = list(range(step, minutes + 1, step))
            delivered = 0.0
            for i, m in enumerate(slots):
                kwh = round(energy_needed / len(slots), 4) if i < len(slots) - 1 else round(energy_needed - delivered, 4)
                delivered += kwh
                emit(vin, fw, "charging_progress", t0 + m * MIN_MS, session_id=session_id,
                     kwh_delta=kwh, power_kw=round(power * rng.uniform(0.85, 1.0), 2))
            soc = soc + delivered / capacity * 100
            emit(vin, fw, "charging_end", t0 + (minutes + 1) * MIN_MS, session_id=session_id, soc=round(soc, 2))

    # OTA campaign
    if rng.random() < 0.7:
        campaign = "OTA-2026-11-A"
        t = START_MS + rng.randint(0, DAYS * DAY_MS - 6 * 3_600_000)
        emit(vin, fw, "ota_status", t, campaign_id=campaign, version="2.6.0", state="downloading")
        t += rng.randint(10, 90) * MIN_MS
        emit(vin, fw, "ota_status", t, campaign_id=campaign, version="2.6.0", state="installing")
        t += rng.randint(15, 45) * MIN_MS
        if rng.random() < 0.12:
            emit(vin, fw, "ota_status", t, campaign_id=campaign, version="2.6.0", state="failed",
                 error_code=rng.choice(["E_CHECKSUM", "E_BATTERY_LOW", "E_TIMEOUT"]))
        elif rng.random() < 0.9:
            emit(vin, fw, "ota_status", t, campaign_id=campaign, version="2.6.0", state="installed")


def main():
    for _ in range(N_VINS):
        simulate_vehicle(make_vin())

    recorded = []
    for e in events:
        latency = rng.randint(200, 3_000)
        if rng.random() < LATE_RATE:
            latency = rng.randint(10, 60) * MIN_MS
        recorded.append({**e, "received_ts": e["event_ts"] + latency})
        if rng.random() < DUPLICATE_RATE:
            recorded.append({**e, "received_ts": e["event_ts"] + latency + rng.randint(1_000, 120_000)})

    recorded.sort(key=lambda e: (e["received_ts"], e["event_id"]))
    out = Path(__file__).resolve().parent.parent / "data" / "events.jsonl"
    with out.open("w") as f:
        for e in recorded:
            f.write(json.dumps(e, separators=(",", ":")) + "\n")
    print(f"wrote {len(recorded)} events for {N_VINS} vehicles to {out}")


if __name__ == "__main__":
    main()
