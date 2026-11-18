package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object DailyDistance extends TelemetryJob {
  val name = "daily_distance"
  val description = "Distance driven and trips per vehicle per UTC day (by trip start)."

  def run(events: DataFrame): DataFrame = {
    Trips.summaries(events)
      .groupBy(col("vin"), Events.day(col("start_ts")).as("day"))
      .agg(sum("distance_km").as("distance_km"), count(lit(1)).as("trips"))
  }
}
