package lucid.telemetry

import org.apache.spark.sql.DataFrame

trait TelemetryJob {
  def name: String
  def description: String
  def run(events: DataFrame): DataFrame
}
