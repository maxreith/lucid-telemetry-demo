package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object TripSummaries extends TelemetryJob {
  val name = "trip_summaries"
  val description = "Distance, duration and average speed per trip."

  def run(events: DataFrame): DataFrame = {
    Trips.summaries(events)
  }
}
