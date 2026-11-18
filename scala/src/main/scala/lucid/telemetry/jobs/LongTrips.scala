package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object LongTrips extends TelemetryJob {
  val name = "long_trips"
  val description = "Trips of 50 km or more."

  def run(events: DataFrame): DataFrame = {
    Trips.summaries(events)
      .filter(col("distance_km") >= 50.0)
      .select("vin", "trip_id", "start_ts", "distance_km")
  }
}
