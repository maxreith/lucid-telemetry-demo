package lucid.telemetry

import lucid.telemetry.jobs._

object Jobs {
  val all: Seq[TelemetryJob] = Seq(
    ChargingSessions,
    ChargingDailyEnergy,
    DcFastChargeCounts,
    ChargingPeakPower,
    BatterySohLatest,
    BatteryTempAlerts,
    CellImbalanceAlerts,
    BatteryHourlyTemp,
    LowSocAlerts,
    DtcCounts,
    CriticalDtcAlerts,
    DtcFirstSeen,
    EcuFaultSummary,
    OtaLatestState,
    OtaCampaignSummary,
    OtaFailures,
    OtaInstallDuration,
    TripSummaries,
    DailyDistance,
    LongTrips,
    TirePressureAlerts,
    TirePressureDailyMin,
    CabinClimateHourly,
    HvacPowerDaily,
    SpeedingEvents,
    LastKnownLocation,
    GeofenceVisits,
    ConnectivityDaily,
    WeakSignalAlerts,
    AdasEventCounts
  )

  val byName: Map[String, TelemetryJob] = all.map(j => j.name -> j).toMap
}
