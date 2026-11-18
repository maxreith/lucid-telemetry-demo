package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object ChargingPeakPower extends TelemetryJob {
  val name = "charging_peak_power"
  val description = "Peak and average charging power per session."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "charging_progress")
      .groupBy("vin", "payload.session_id")
      .agg(max("payload.power_kw").as("peak_power_kw"), avg("payload.power_kw").as("avg_power_kw"))
  }
}
