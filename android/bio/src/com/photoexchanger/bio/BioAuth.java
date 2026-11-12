package com.photoexchanger.bio;

import android.app.Activity;
import android.content.Context;
import android.content.DialogInterface;
import android.content.pm.PackageManager;
import android.hardware.biometrics.BiometricManager;
import android.hardware.biometrics.BiometricPrompt;
import android.os.Build;
import android.os.CancellationSignal;

/**
 * Tiny BiometricPrompt bridge for the Rust app.
 *
 * BiometricPrompt.AuthenticationCallback is an abstract class, which cannot be
 * implemented from pure JNI. This class is compiled to classes.dex
 * (see build_dex.sh), embedded into the Rust binary and loaded at runtime with
 * InMemoryDexClassLoader, so the generated Gradle project needs no changes.
 *
 * Rust starts a prompt with {@link #start} and polls {@link #result}.
 */
public final class BioAuth extends BiometricPrompt.AuthenticationCallback
        implements Runnable, DialogInterface.OnClickListener {

    public static final int PENDING = 0;
    public static final int SUCCESS = 1;
    public static final int FAILED = 2;
    public static final int UNAVAILABLE = 3;

    /** Written on the main thread, polled from the Rust thread. */
    public volatile int result = PENDING;

    private final Activity activity;
    private final String title;
    private final String negative;
    private final CancellationSignal cancelSignal = new CancellationSignal();

    private BioAuth(Activity activity, String title, String negative) {
        this.activity = activity;
        this.title = title;
        this.negative = negative;
    }

    /** True when a strong biometric is enrolled and usable. */
    public static boolean available(Context c) {
        if (Build.VERSION.SDK_INT < 28) return false;
        if (Build.VERSION.SDK_INT >= 30) {
            BiometricManager bm = c.getSystemService(BiometricManager.class);
            return bm != null
                    && bm.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)
                    == BiometricManager.BIOMETRIC_SUCCESS;
        }
        if (Build.VERSION.SDK_INT == 29) {
            BiometricManager bm = c.getSystemService(BiometricManager.class);
            return bm != null && bm.canAuthenticate() == BiometricManager.BIOMETRIC_SUCCESS;
        }
        return c.getPackageManager().hasSystemFeature(PackageManager.FEATURE_FINGERPRINT);
    }

    /** Shows the system biometric prompt on the UI thread. */
    public static BioAuth start(Activity activity, String title, String negative) {
        BioAuth b = new BioAuth(activity, title, negative);
        if (Build.VERSION.SDK_INT < 28) {
            b.result = UNAVAILABLE;
        } else {
            activity.runOnUiThread(b);
        }
        return b;
    }

    /** Dismisses a pending prompt. */
    public void cancel() {
        cancelSignal.cancel();
    }

    @Override
    public void run() {
        try {
            BiometricPrompt.Builder builder = new BiometricPrompt.Builder(activity)
                    .setTitle(title)
                    .setNegativeButton(negative, activity.getMainExecutor(), this);
            if (Build.VERSION.SDK_INT >= 30) {
                builder.setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG);
            }
            builder.build().authenticate(cancelSignal, activity.getMainExecutor(), this);
        } catch (Throwable t) {
            result = UNAVAILABLE;
        }
    }

    @Override
    public void onAuthenticationSucceeded(BiometricPrompt.AuthenticationResult r) {
        result = SUCCESS;
    }

    /** Unrecoverable error or user cancellation. */
    @Override
    public void onAuthenticationError(int code, CharSequence msg) {
        result = FAILED;
    }

    /** Negative ("Cancel") button. */
    @Override
    public void onClick(DialogInterface dialog, int which) {
        result = FAILED;
    }
    // onAuthenticationFailed (a single non-matching attempt) keeps the prompt open.
}
