package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object ChargingDailyEnergy extends TelemetryJob {
  val name = "charging_daily_energy"
  val description = "Energy charged per vehicle per UTC day."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "charging_progress")
      .groupBy(col("vin"), Events.day(col("event_ts")).as("day"))
      .agg(sum("payload.kwh_delta").as("energy_kwh"), countDistinct("payload.session_id").as("sessions"))
  }
}
