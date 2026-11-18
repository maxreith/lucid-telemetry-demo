package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object LastKnownLocation extends TelemetryJob {
  val name = "last_known_location"
  val description = "Most recent location per vehicle, by event time."

  def run(events: DataFrame): DataFrame = {
    Events.latestBy(Events.ofType(Events.dedup(events), "location"), "vin")
      .select(col("vin"), col("payload.lat").as("lat"), col("payload.lon").as("lon"), col("event_ts"))
  }
}
