package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object HvacPowerDaily extends TelemetryJob {
  val name = "hvac_power_daily"
  val description = "Average and peak HVAC power per vehicle per UTC day."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "climate")
      .groupBy(col("vin"), Events.day(col("event_ts")).as("day"))
      .agg(avg("payload.hvac_power_kw").as("avg_hvac_power_kw"), max("payload.hvac_power_kw").as("max_hvac_power_kw"))
  }
}
