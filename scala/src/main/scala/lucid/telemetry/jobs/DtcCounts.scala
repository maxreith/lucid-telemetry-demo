package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object DtcCounts extends TelemetryJob {
  val name = "dtc_counts"
  val description = "Number of occurrences of each diagnostic trouble code per vehicle."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "diagnostic")
      .groupBy("vin", "payload.code")
      .agg(count(lit(1)).as("occurrences"))
  }
}
