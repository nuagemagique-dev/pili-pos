# R8 consumer rules of :pos-core (DECISIONS 11.7, proven necessary by spike S2).
-dontwarn java.awt.*
-keep class com.sun.jna.** { *; }
-keep class uniffi.pos_ffi.** { *; }
-keep interface * extends com.sun.jna.Library
