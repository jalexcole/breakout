#include "raylib.h"
#include "Entity.hpp"
#include "Player.hpp"
#include <iostream>
#include <cmath>

Player::Player(Vector2 vector2, int length, int width) : Entity(vector2, length, width) {
    init();
}

void Player::init() {
    acceleration = {0, 0};
    velocity = {0,0};
}

void Player::update() {
    updatePosition();
    updateRectangle();
    updateVelocity();
}

void Player::checkInput() {
    float accelerationValue = .1;
    float maxAcceleration = .3;
    if (IsKeyDown(KEY_LEFT) || IsKeyDown(KEY_A)) {
        acceleration.x -= accelerationValue;
        if (acceleration.x < -1 * maxAcceleration) {
            acceleration.x = -1 * maxAcceleration;
        }
    } else if (IsKeyDown(KEY_RIGHT) || IsKeyDown(KEY_D)) {
        acceleration.x += accelerationValue;

        if (acceleration.x >  maxAcceleration) {
            acceleration.x = maxAcceleration;
        }
    } else {
        acceleration.x = 0;
    }

    if (acceleration.x == 0 && velocity.x != 0) {
        if (velocity.x > 0) {
            velocity.x -= accelerationValue * 2;
        } else if (velocity.x < 0) {
            velocity.x += accelerationValue * 2;
        }
    }

    if (acceleration.x == 0 && (abs(velocity.x) < 2)) {
        velocity.x = 0;
    }


}
    
void Player::preventLeft() {        
    if (velocity.x < 0) {
        velocity.x *= -1 / 2;
        }
}

void Player::preventRight() {
    if (velocity.x > 0) {
        velocity.x *= -1 / 2;
    }
}

   
void Player::updateVelocity() {
    velocity.x += acceleration.x;
    velocity.y += acceleration.y;
}

    
