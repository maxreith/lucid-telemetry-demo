package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object ChargingSessions extends TelemetryJob {
  val name = "charging_sessions"
  val description = "One record per charging session: duration, energy delivered, state of charge and peak power."

  def run(events: DataFrame): DataFrame = {
    val charging = Events.ofType(events, "charging_start", "charging_progress", "charging_end")
    def when_(t: String, c: String) = when(col("type") === t, col(c))
    charging
      .groupBy("vin", "payload.session_id")
      .agg(
        min(when_("charging_start", "event_ts")).as("start_ts"),
        max(when_("charging_end", "event_ts")).as("end_ts"),
        first(when_("charging_start", "payload.soc"), ignoreNulls = true).as("soc_start"),
        max(when_("charging_end", "payload.soc")).as("soc_end"),
        sum(when_("charging_progress", "payload.kwh_delta")).as("energy_kwh"),
        max(when_("charging_progress", "payload.power_kw")).as("peak_power_kw"),
        first(when_("charging_start", "payload.charger_type"), ignoreNulls = true).as("charger_type"),
        first(when_("charging_start", "payload.pack_capacity_kwh"), ignoreNulls = true).as("pack_capacity_kwh")
      )
      .filter(col("start_ts").isNotNull && col("end_ts").isNotNull)
      .withColumn("duration_min", (col("end_ts") - col("start_ts")) / 60000.0)
      .select("vin", "session_id", "start_ts", "end_ts", "duration_min", "soc_start", "soc_end", "energy_kwh",
        "peak_power_kw", "charger_type", "pack_capacity_kwh")
  }
}
