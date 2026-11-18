package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object TirePressureDailyMin extends TelemetryJob {
  val name = "tire_pressure_daily_min"
  val description = "Lowest pressure per tire per vehicle per UTC day."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "tire_pressure")
      .groupBy(col("vin"), Events.day(col("event_ts")).as("day"))
      .agg(min("payload.fl").as("min_fl_kpa"), min("payload.fr").as("min_fr_kpa"),
        min("payload.rl").as("min_rl_kpa"), min("payload.rr").as("min_rr_kpa"))
  }
}
