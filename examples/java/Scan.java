// A runnable Java example: scan a small universe through the binding.
//
//   cargo build -p wickra-screener-c
//   mvn -f bindings/java/pom.xml -q package -DskipTests
//   javac -cp bindings/java/target/classes examples/java/Scan.java -d examples/java/out
//   java --enable-native-access=ALL-UNNAMED \
//        -Dnative.lib.dir=target/debug \
//        -cp "bindings/java/target/classes;examples/java/out" Scan
import org.wickra.screener.Screener;

public final class Scan {
    private static final String SPEC =
            "{\"universe\":[\"AAA\",\"BBB\"],\"condition\":{\"type\":\"cmp\","
                    + "\"left\":{\"kind\":\"price\",\"field\":\"close\"},\"op\":\"gt\","
                    + "\"right\":{\"kind\":\"const\",\"value\":10.0}}}";

    private static String candle(int close) {
        return "{\"time\":1,\"open\":" + close + ",\"high\":" + close
                + ",\"low\":" + close + ",\"close\":" + close + ",\"volume\":1}";
    }

    public static void main(String[] args) {
        try (Screener screener = new Screener(SPEC)) {
            String cmd = "{\"cmd\":\"scan\",\"data\":{"
                    + "\"AAA\":[" + candle(5) + "],"
                    + "\"BBB\":[" + candle(15) + "]}}";
            String response = screener.command(cmd);
            System.out.println("wickra-screener " + Screener.version());
            System.out.println(response);
        }
    }
}
