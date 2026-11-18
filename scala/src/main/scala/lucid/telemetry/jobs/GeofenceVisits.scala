package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object GeofenceVisits extends TelemetryJob {
  val name = "geofence_visits"
  val description = "Location pings inside the Newark, CA headquarters geofence per vehicle."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "location")
      .filter(col("payload.lat").between(37.50, 37.56) && col("payload.lon").between(-122.08, -122.00))
      .groupBy("vin")
      .agg(count(lit(1)).as("pings"), min("event_ts").as("first_ts"), max("event_ts").as("last_ts"))
  }
}
